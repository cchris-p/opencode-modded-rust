/**
 * Bundled auth plugin for OpenAI/Codex.
 *
 * Matches vanilla OpenCode's OpenAI auth surface: ChatGPT Pro/Plus browser
 * login (PKCE + localhost callback), ChatGPT Pro/Plus headless login (device
 * code polling), and manual API key entry. For OAuth auth the loader returns a
 * custom fetch that rewrites OpenAI responses/chat-completions requests to the
 * Codex backend and injects the stored ChatGPT credentials.
 */

const CLIENT_ID = "app_EMoamEEZ73f0CkXaXp7hrann";
const ISSUER = "https://auth.openai.com";
const CODEX_API_ENDPOINT = "https://chatgpt.com/backend-api/codex/responses";
const OAUTH_PORT = 1455;
const OAUTH_POLLING_SAFETY_MARGIN_MS = 3000;
const OAUTH_DUMMY_KEY = "opencode-oauth-dummy-key";
const USER_AGENT = `opencode/${process.env.OPENCODE_VERSION ?? "rust"}`;

interface PkceCodes {
  verifier: string;
  challenge: string;
}

interface TokenResponse {
  id_token?: string;
  access_token: string;
  refresh_token: string;
  expires_in?: number;
}

interface StoredAuth {
  type?: string;
  access?: string;
  refresh?: string;
  expires?: number;
  accountId?: string;
  account_id?: string;
}

interface OAuthMethod {
  type: string;
  label: string;
}

function base64UrlEncode(input: Uint8Array): string {
  return Buffer.from(input)
    .toString("base64")
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");
}

async function generatePKCE(): Promise<PkceCodes> {
  const crypto = await import("node:crypto");
  const verifier = base64UrlEncode(crypto.randomBytes(32));
  const challenge = base64UrlEncode(
    crypto.createHash("sha256").update(verifier).digest(),
  );
  return { verifier, challenge };
}

function parseJwtClaims(token: string): Record<string, any> | undefined {
  const parts = token.split(".");
  if (parts.length !== 3) return undefined;
  try {
    return JSON.parse(Buffer.from(parts[1], "base64url").toString());
  } catch {
    return undefined;
  }
}

function extractAccountIdFromClaims(claims: Record<string, any>): string | undefined {
  return (
    claims.chatgpt_account_id ||
    claims["https://api.openai.com/auth"]?.chatgpt_account_id ||
    claims.organizations?.[0]?.id
  );
}

function extractAccountId(tokens: TokenResponse): string | undefined {
  if (tokens.id_token) {
    const claims = parseJwtClaims(tokens.id_token);
    const accountId = claims && extractAccountIdFromClaims(claims);
    if (accountId) return accountId;
  }
  if (tokens.access_token) {
    const claims = parseJwtClaims(tokens.access_token);
    return claims ? extractAccountIdFromClaims(claims) : undefined;
  }
  return undefined;
}

function extractResidency(token: string): string | undefined {
  const claims = parseJwtClaims(token);
  const residency =
    claims?.["https://api.openai.com/auth"]?.chatgpt_compute_residency ??
    claims?.chatgpt_compute_residency;
  if (!residency || residency === "no_constraint") return undefined;
  return residency;
}

function buildAuthorizeUrl(redirectUri: string, pkce: PkceCodes, state: string): string {
  const params = new URLSearchParams({
    response_type: "code",
    client_id: CLIENT_ID,
    redirect_uri: redirectUri,
    scope: "openid profile email offline_access",
    code_challenge: pkce.challenge,
    code_challenge_method: "S256",
    id_token_add_organizations: "true",
    codex_cli_simplified_flow: "true",
    state,
    originator: "opencode",
  });
  return `${ISSUER}/oauth/authorize?${params.toString()}`;
}

async function exchangeCodeForTokens(
  code: string,
  redirectUri: string,
  pkce: PkceCodes,
): Promise<TokenResponse> {
  const response = await fetch(`${ISSUER}/oauth/token`, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "authorization_code",
      code,
      redirect_uri: redirectUri,
      client_id: CLIENT_ID,
      code_verifier: pkce.verifier,
    }).toString(),
  });
  if (!response.ok) {
    throw new Error(`Token exchange failed: ${response.status}`);
  }
  return (await response.json()) as TokenResponse;
}

async function refreshAccessToken(refreshToken: string): Promise<TokenResponse> {
  const response = await fetch(`${ISSUER}/oauth/token`, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "refresh_token",
      refresh_token: refreshToken,
      client_id: CLIENT_ID,
    }).toString(),
  });
  if (!response.ok) {
    throw new Error(`Token refresh failed: ${response.status}`);
  }
  return (await response.json()) as TokenResponse;
}

interface PendingOAuth {
  pkce: PkceCodes;
  state: string;
  resolve: (tokens: TokenResponse) => void;
  reject: (error: Error) => void;
}

let oauthServer: any;
let pendingOAuth: PendingOAuth | undefined;

function htmlPage(title: string, message: string): string {
  return `<!doctype html><html><head><meta charset="utf-8"><title>${title}</title></head><body style="font-family: sans-serif; padding: 2rem;"><h1>${title}</h1><p>${message}</p></body></html>`;
}

async function startOAuthServer(): Promise<string> {
  if (oauthServer) {
    return `http://localhost:${OAUTH_PORT}/auth/callback`;
  }

  const http = await import("node:http");
  const redirectUri = `http://localhost:${OAUTH_PORT}/auth/callback`;

  oauthServer = http.createServer((req: any, res: any) => {
    const url = new URL(req.url || "/", `http://localhost:${OAUTH_PORT}`);

    if (url.pathname === "/auth/callback") {
      const code = url.searchParams.get("code");
      const state = url.searchParams.get("state");
      const error = url.searchParams.get("error");
      const errorDescription = url.searchParams.get("error_description");

      if (error) {
        const errorMsg = errorDescription || error;
        pendingOAuth?.reject(new Error(errorMsg));
        pendingOAuth = undefined;
        res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
        res.end(htmlPage("ChatGPT authorization failed", errorMsg));
        return;
      }

      if (!code) {
        const errorMsg = "Missing authorization code";
        pendingOAuth?.reject(new Error(errorMsg));
        pendingOAuth = undefined;
        res.writeHead(400, { "Content-Type": "text/html; charset=utf-8" });
        res.end(htmlPage("ChatGPT authorization failed", errorMsg));
        return;
      }

      if (!pendingOAuth || state !== pendingOAuth.state) {
        const errorMsg = "Invalid state - potential CSRF attack";
        pendingOAuth?.reject(new Error(errorMsg));
        pendingOAuth = undefined;
        res.writeHead(400, { "Content-Type": "text/html; charset=utf-8" });
        res.end(htmlPage("ChatGPT authorization failed", errorMsg));
        return;
      }

      const current = pendingOAuth;
      pendingOAuth = undefined;

      exchangeCodeForTokens(code, redirectUri, current.pkce)
        .then((tokens) => current.resolve(tokens))
        .catch((err) => current.reject(err));

      res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      res.end(
        htmlPage(
          "ChatGPT authorization complete",
          "You can close this window and return to opencode.",
        ),
      );
      return;
    }

    if (url.pathname === "/cancel") {
      pendingOAuth?.reject(new Error("Login cancelled"));
      pendingOAuth = undefined;
      res.writeHead(200);
      res.end("Login cancelled");
      return;
    }

    res.writeHead(404);
    res.end("Not found");
  });

  await new Promise<void>((resolve, reject) => {
    oauthServer.listen(OAUTH_PORT, () => resolve());
    oauthServer.on("error", reject);
  });

  return redirectUri;
}

function stopOAuthServer(): void {
  if (oauthServer) {
    oauthServer.close(() => {});
    oauthServer = undefined;
  }
}

function waitForOAuthCallback(pkce: PkceCodes, state: string): Promise<TokenResponse> {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      if (pendingOAuth) {
        pendingOAuth = undefined;
        reject(new Error("OAuth callback timeout - authorization took too long"));
      }
    }, 5 * 60 * 1000);

    pendingOAuth = {
      pkce,
      state,
      resolve: (tokens) => {
        clearTimeout(timeout);
        resolve(tokens);
      },
      reject: (error) => {
        clearTimeout(timeout);
        reject(error);
      },
    };
  });
}

async function browserAuthorize() {
  const redirectUri = await startOAuthServer();
  const pkce = await generatePKCE();
  const crypto = await import("node:crypto");
  const state = base64UrlEncode(crypto.randomBytes(32));
  const authUrl = buildAuthorizeUrl(redirectUri, pkce, state);
  const callbackPromise = waitForOAuthCallback(pkce, state);

  return {
    url: authUrl,
    instructions: "Complete authorization in your browser.",
    method: "auto" as const,
    callback: async () => {
      const tokens = await callbackPromise;
      stopOAuthServer();
      return {
        type: "success" as const,
        refresh: tokens.refresh_token,
        access: tokens.access_token,
        expires: Date.now() + (tokens.expires_in ?? 3600) * 1000,
        accountId: extractAccountId(tokens),
      };
    },
  };
}

async function headlessAuthorize() {
  const deviceResponse = await fetch(`${ISSUER}/api/accounts/deviceauth/usercode`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "User-Agent": USER_AGENT,
    },
    body: JSON.stringify({ client_id: CLIENT_ID }),
  });

  if (!deviceResponse.ok) {
    throw new Error("Failed to initiate device authorization");
  }

  const deviceData = (await deviceResponse.json()) as {
    device_auth_id: string;
    user_code: string;
    interval: string;
  };
  const interval = Math.max(parseInt(deviceData.interval) || 5, 1) * 1000;

  return {
    url: `${ISSUER}/codex/device`,
    instructions: `Enter code: ${deviceData.user_code}`,
    method: "auto" as const,
    callback: async () => {
      while (true) {
        const response = await fetch(`${ISSUER}/api/accounts/deviceauth/token`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "User-Agent": USER_AGENT,
          },
          body: JSON.stringify({
            device_auth_id: device_data.device_auth_id,
            user_code: device_data.user_code,
          }),
        });

        if (response.ok) {
          const data = (await response.json()) as {
            authorization_code: string;
            code_verifier: string;
          };

          const tokenResponse = await fetch(`${ISSUER}/oauth/token`, {
            method: "POST",
            headers: { "Content-Type": "application/x-www-form-urlencoded" },
            body: new URLSearchParams({
              grant_type: "authorization_code",
              code: data.authorization_code,
              redirect_uri: `${ISSUER}/deviceauth/callback`,
              client_id: CLIENT_ID,
              code_verifier: data.code_verifier,
            }).toString(),
          });

          if (!tokenResponse.ok) {
            throw new Error(`Token exchange failed: ${tokenResponse.status}`);
          }

          const tokens = (await tokenResponse.json()) as TokenResponse;

          return {
            type: "success" as const,
            refresh: tokens.refresh_token,
            access: tokens.access_token,
            expires: Date.now() + (tokens.expires_in ?? 3600) * 1000,
            accountId: extractAccountId(tokens),
          };
        }

        if (response.status !== 403 && response.status !== 404) {
          return { type: "failed" as const };
        }

        await new Promise((resolve) =>
          setTimeout(resolve, interval + OAUTH_POLLING_SAFETY_MARGIN_MS),
        );
      }
    },
  };
}

function accountIdOf(auth: StoredAuth | undefined): string | undefined {
  return auth?.accountId ?? auth?.account_id;
}

function createOAuthFetch(initialAuth: StoredAuth) {
  let currentAuth = initialAuth;
  let refreshPromise: Promise<{ access: string; accountId?: string }> | undefined;

  return async (requestInput: string | URL | Request, init?: RequestInit) => {
    const parsed =
      requestInput instanceof URL
        ? requestInput
        : new URL(
            typeof requestInput === "string" ? requestInput : requestInput.url,
          );

    if (!currentAuth.access || (currentAuth.expires ?? 0) < Date.now()) {
      if (!refreshPromise) {
        refreshPromise = refreshAccessToken(currentAuth.refresh ?? "")
          .then((tokens) => {
            const accountId = extractAccountId(tokens) || accountIdOf(currentAuth);
            currentAuth = {
              ...currentAuth,
              access: tokens.access_token,
              refresh: tokens.refresh_token,
              expires: Date.now() + (tokens.expires_in ?? 3600) * 1000,
              accountId,
            };
            return { access: tokens.access_token, accountId };
          })
          .finally(() => {
            refreshPromise = undefined;
          });
      }
      const refreshed = await refreshPromise;
      currentAuth = { ...currentAuth, access: refreshed.access };
    }

    const headers = new Headers();
    const existing = (init?.headers ?? {}) as Record<string, string>;
    for (const [key, value] of Object.entries(existing)) {
      if (key.toLowerCase() === "authorization") continue;
      if (value !== undefined) headers.set(key, String(value));
    }
    headers.set("authorization", `Bearer ${currentAuth.access}`);
    const accountId = accountIdOf(currentAuth);
    if (accountId) {
      headers.set("ChatGPT-Account-Id", accountId);
    }

    const rewrite =
      parsed.pathname.includes("/v1/responses") ||
      parsed.pathname.includes("/chat/completions");
    let url: URL | string = parsed;
    if (rewrite) {
      url = new URL(CODEX_API_ENDPOINT);
      const residency = extractResidency(currentAuth.access ?? "");
      if (residency) {
        headers.set("x-openai-internal-codex-residency", residency);
      }
    }

    return fetch(url, {
      ...init,
      headers,
    });
  };
}

export default async function CodexAuthPlugin() {
  return {
    auth: {
      provider: "openai",
      methods: [
        {
          type: "oauth",
          label: "ChatGPT Pro/Plus (browser)",
        },
        {
          type: "oauth",
          label: "ChatGPT Pro/Plus (headless)",
        },
        {
          type: "api",
          label: "Manually enter API Key",
        },
      ],
      async authorize(method: OAuthMethod) {
        const label = (method?.label ?? "").toLowerCase();
        if (label.includes("headless")) {
          return await headlessAuthorize();
        }
        return await browserAuthorize();
      },
      async loader(getAuth?: () => Promise<StoredAuth | undefined>) {
        const auth = getAuth ? await getAuth() : undefined;
        if (!auth || auth.type !== "oauth") return {};
        return {
          apiKey: OAUTH_DUMMY_KEY,
          fetch: createOAuthFetch(auth),
        };
      },
    },
  };
}
