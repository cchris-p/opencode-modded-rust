---
id: "FEAT-079"
title: "Integrate SketchVLM visual reasoning into ORT"
priority: "P2"
type: "feature"
area: "FEAT"
spec: ""
status: "todo"
created: "2026-11-13"
---

# Integrate SketchVLM visual reasoning into ORT

## Summary

Evaluate and integrate SketchVLM — a vision model that draws its reasoning directly onto the image (sketching lines, circles, and arrows) — so ORT can surface visual, inspectable reasoning for image and agent tasks instead of opaque text-only answers.

Source material:
- Demo video: https://www.tiktok.com/t/ZPLWHm996/ (creator: Github Signals, `@github.signals`)
- Upstream repo: https://github.com/brandon-collins7/sketchvlm

## Transcript (auto-generated captions from the source video)

> SketchVLM is the vision model that draws its reasoning directly onto the screen.
> Most artificial intelligence simply types out its answer, leaving you to guess if it actually understands the image.
> This new framework changes that by having the model sketch lines, circles and arrows right on the picture to explain exactly where it is looking.
> By visually tracing a path through a maze or circling objects in a photo, the system makes its thinking visible.
> You can immediately see if the logic makes sense or if the model got lost.
> It turns a black box into a clear step by step visual argument.

(Transcript taken from the video's ASR caption track; "Sketchblm" in the raw captions is SketchVLM.)

## Why this exists

ORT currently exposes reasoning as text/tool-call traces. A visual reasoning overlay would make model behavior auditable for image-heavy work and demo/QA flows, directly serving explainability for agent decisions.

## Scope

- Evaluate the upstream `sketchvlm` model/repo: license, runtime deps, hardware requirements, and output format for sketches/annotations.
- Define an integration boundary (provider or tool) that accepts an image + prompt and returns mentally annotated overlay artifacts (paths, boxes, arrows, labels).
- Render returned annotations in ORT's existing visualization/output surface (no new UI stack unless justified).
- Provide at least one end-to-end demo (e.g., maze path tracing or object circling) with reproducible inputs.
- Document the API/handle so other agents can request visual reasoning as evidence.
- Add a configuration/kill-switch path so the capability is opt-in and does not affect default text-only sessions.

## Non-goals

- Training or fine-tuning a new vision model.
- Replacing the existing text reasoning/tool-call trace.
- A full annotation editor UI; read-only overlay rendering is sufficient for the first pass.

## Open questions

- Is the upstream model runnable locally under the fleet's GPU/host constraints, or does it require offload (see `vastai-gpu-offload` skill)?
- Which ORT surface owns image artifacts today (artifact store vs. inline output)?
- License compatibility with ORT distribution and standards-managed vendor rules.

## Done when

- Evaluation notes recorded (license, deps, hardware, sample outputs).
- At least one reproducible visual-reasoning demo wired into ORT behind an opt-in flag.
- API/handle documented and the capability listed in the relevant provider/tool inventory.
- Default sessions remain unaffected when the feature is disabled.
