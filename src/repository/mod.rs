pub(crate) mod compare;
pub(crate) mod discovery;
pub(crate) mod objects;
pub(crate) mod plan;
pub(crate) mod sync;

pub use compare::{compare_postgres_with_inventory, CompareReport};
pub use plan::{
    plan_postgres_with_inventory, CompareSummary, DependencyWarning, PlanItem, PlanReport,
    PlanSelection,
};
pub use sync::{
    export_postgres_with_inventory, sync_postgres_with_inventory, ExportReport, ExportSelection,
    SyncReport,
};

pub(crate) use compare::compare_postgres_command;
pub(crate) use objects::{
    ensure_database_object_path, enum_file_path, extension_file_path, function_identity_slug,
    index_file_path, safe_file_component, schema_file_path, sequence_file_path, table_file_path,
    trigger_file_path, view_file_path, ObjectRef,
};
pub(crate) use plan::{plan_postgres_command, PlanColumn, TableDifferenceAnalysis};
pub(crate) use sync::{export_postgres_command, sync_postgres_command};
