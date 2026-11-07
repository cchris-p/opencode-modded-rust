//! Raw `scopemux-core` FFI surface.
//!
//! Only `#[repr(C)]` mirrors of the core structs and the `extern "C"`
//! declarations live here. No safe wrapper, policy, or provider code.

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct ProjectContext {
    _private: [u8; 0],
}

#[repr(C)]
pub struct ProjectInfoBlock {
    pub id: *mut c_char,
    pub name: *mut c_char,
    pub qualified_name: *mut c_char,
    pub file_path: *mut c_char,
    pub node: *mut c_void,
    pub node_type: c_int,
    pub language: c_int,
    pub kind: c_int,
    pub observability_kind: c_int,
    pub tier: c_int,
    pub estimated_tokens: usize,
    pub related_symbol_count: usize,
    pub origin: c_int,
    pub lifecycle: c_int,
    pub provenance: *mut c_char,
    pub confidence: f32,
    pub plan_kind: c_int,
    pub desired_shape: *mut c_char,
    pub rationale: *mut c_char,
    pub anchor_list: *mut c_char,
}

#[repr(C)]
pub struct ProjectPlanNode {
    pub id: *mut c_char,
    pub task_id: *mut c_char,
    pub slug: *mut c_char,
    pub title: *mut c_char,
    pub desired_shape: *mut c_char,
    pub rationale: *mut c_char,
    pub provenance: *mut c_char,
    pub projected_symbol: *mut c_char,
    pub file_path: *mut c_char,
    pub kind: c_int,
    pub observability_kind: c_int,
    pub lifecycle: c_int,
    pub confidence: f32,
    pub anchor_ids: *mut *mut c_char,
    pub anchor_count: usize,
    pub anchor_capacity: usize,
}

#[repr(C)]
pub struct ProjectPlanNodeReconciliationEntry {
    pub node: *const ProjectPlanNode,
    pub previous_lifecycle: c_int,
    pub new_lifecycle: c_int,
}

#[repr(C)]
pub struct ProjectPlanNodeReconciliationResult {
    pub entries: *mut ProjectPlanNodeReconciliationEntry,
    pub entry_count: usize,
    pub stale_count: usize,
    pub conflict_count: usize,
    pub implemented_count: usize,
}

#[repr(C)]
pub struct ProjectSearchRequest {
    pub query_text: *const c_char,
    pub anchor_symbol: *const c_char,
    pub anchor_file_path: *const c_char,
    pub min_tier: c_int,
    pub max_tier: c_int,
    pub include_related: bool,
    pub include_dependencies: bool,
    pub max_hits: usize,
    pub origin_mask: u32,
    pub lifecycle_mask: u32,
}

#[repr(C)]
pub struct ProjectSearchHit {
    pub block: *const ProjectInfoBlock,
    pub score: usize,
    pub name_match: bool,
    pub text_match: bool,
    pub relationship_match: bool,
}

#[repr(C)]
pub struct ProjectSearchResult {
    pub hits: *mut ProjectSearchHit,
    pub hit_count: usize,
    pub total_match_count: usize,
}

// WI-036 delta view. Layout mirrors `ProjectDeltaEntry`/`ProjectDeltaResult`.
#[repr(C)]
pub struct ProjectDeltaEntry {
    pub kind: c_int,
    pub block: *const ProjectInfoBlock,
    pub plan_node: *const ProjectPlanNode,
    pub anchors: *const c_char,
    pub projected_shape: *const c_char,
    pub provenance: *const c_char,
    pub confidence: f32,
    pub lifecycle: c_int,
    pub estimated_tokens: usize,
}

#[repr(C)]
pub struct ProjectDeltaResult {
    pub task_id: *const c_char,
    pub stage: *const c_char,
    pub entries: *mut ProjectDeltaEntry,
    pub entry_count: usize,
    pub add_count: usize,
    pub change_count: usize,
    pub remove_count: usize,
    pub reuse_count: usize,
    pub estimated_tokens: usize,
}

// WI-036 map query surface. Layout mirrors `ProjectMapResultItem`/
// `ProjectMapQueryResult`.
#[repr(C)]
pub struct ProjectMapResultItem {
    pub block: *const ProjectInfoBlock,
    pub kind: c_int,
    pub reason: *const c_char,
    pub provenance: *const c_char,
    pub confidence: f32,
    pub estimated_tokens: usize,
    pub distance: usize,
}

#[repr(C)]
pub struct ProjectMapQueryResult {
    pub kind: c_int,
    pub items: *mut ProjectMapResultItem,
    pub item_count: usize,
    pub estimated_tokens: usize,
}

extern "C" {
    pub fn project_context_create(root_directory: *const c_char) -> *mut ProjectContext;
    pub fn project_context_free(project: *mut ProjectContext);
    pub fn project_update_file(
        project: *mut ProjectContext,
        filepath: *const c_char,
        language: c_int,
        out_changed: *mut bool,
    ) -> bool;
    pub fn project_context_remove_file(
        project: *mut ProjectContext,
        filepath: *const c_char,
    ) -> bool;
    pub fn project_context_rebuild_ir(project: *mut ProjectContext) -> bool;
    pub fn project_context_last_recomputed_file_count(project: *const ProjectContext) -> usize;
    pub fn project_context_last_rebuild_file_count(project: *const ProjectContext) -> usize;
    pub fn project_context_last_rebuild_was_incremental(project: *const ProjectContext) -> bool;
    pub fn project_context_last_info_block_reused_file_count(
        project: *const ProjectContext,
    ) -> usize;
    pub fn project_context_last_info_block_recomputed_file_count(
        project: *const ProjectContext,
    ) -> usize;
    pub fn project_add_file(
        project: *mut ProjectContext,
        filepath: *const c_char,
        language: c_int,
    ) -> bool;
    pub fn project_parse_all_files(project: *mut ProjectContext) -> bool;
    pub fn project_resolve_references(project: *mut ProjectContext) -> bool;
    pub fn project_context_rebuild_info_blocks(project: *mut ProjectContext) -> bool;
    pub fn project_context_search_info_blocks(
        project: *mut ProjectContext,
        request: *const ProjectSearchRequest,
        out_result: *mut ProjectSearchResult,
    ) -> bool;
    pub fn project_search_result_free(result: *mut ProjectSearchResult);
    pub fn project_context_plan_node_create(
        project: *mut ProjectContext,
        task_id: *const c_char,
        slug: *const c_char,
        kind: c_int,
    ) -> *mut ProjectPlanNode;
    pub fn project_context_plan_node_set_title(
        project: *mut ProjectContext,
        node: *mut ProjectPlanNode,
        title: *const c_char,
    ) -> bool;
    pub fn project_context_plan_node_set_desired_shape(
        project: *mut ProjectContext,
        node: *mut ProjectPlanNode,
        desired_shape: *const c_char,
    ) -> bool;
    pub fn project_context_plan_node_set_rationale(
        project: *mut ProjectContext,
        node: *mut ProjectPlanNode,
        rationale: *const c_char,
    ) -> bool;
    pub fn project_context_plan_node_set_provenance(
        project: *mut ProjectContext,
        node: *mut ProjectPlanNode,
        provenance: *const c_char,
    ) -> bool;
    pub fn project_context_plan_node_set_projected_symbol(
        project: *mut ProjectContext,
        node: *mut ProjectPlanNode,
        symbol_name: *const c_char,
    ) -> bool;
    pub fn project_context_plan_node_set_file_path(
        project: *mut ProjectContext,
        node: *mut ProjectPlanNode,
        file_path: *const c_char,
    ) -> bool;
    pub fn project_context_plan_node_set_lifecycle(
        project: *mut ProjectContext,
        node: *mut ProjectPlanNode,
        lifecycle: c_int,
    ) -> bool;
    pub fn project_context_plan_node_add_anchor(
        project: *mut ProjectContext,
        node: *mut ProjectPlanNode,
        anchor_block_id: *const c_char,
    ) -> bool;
    pub fn project_context_reconcile_plan_nodes(
        project: *mut ProjectContext,
        out_result: *mut ProjectPlanNodeReconciliationResult,
    ) -> bool;
    pub fn project_plan_node_reconciliation_result_free(
        result: *mut ProjectPlanNodeReconciliationResult,
    );
    pub fn project_context_compute_delta(
        project: *mut ProjectContext,
        task_id: *const c_char,
        stage: *const c_char,
        out_result: *mut ProjectDeltaResult,
    ) -> bool;
    pub fn project_context_query_resolve(
        project: *mut ProjectContext,
        task_id: *const c_char,
        stage: *const c_char,
        out_result: *mut ProjectMapQueryResult,
    ) -> bool;
    pub fn project_context_query_duplicates(
        project: *mut ProjectContext,
        scope: *const c_char,
        out_result: *mut ProjectMapQueryResult,
    ) -> bool;
    pub fn project_context_query_observability(
        project: *mut ProjectContext,
        symbol: *const c_char,
        out_result: *mut ProjectMapQueryResult,
    ) -> bool;
    pub fn project_context_query_change_impact(
        project: *mut ProjectContext,
        files: *const *const c_char,
        file_count: usize,
        out_result: *mut ProjectMapQueryResult,
    ) -> bool;
    pub fn project_map_query_result_free(result: *mut ProjectMapQueryResult);
    pub fn project_delta_result_free(result: *mut ProjectDeltaResult);
}
