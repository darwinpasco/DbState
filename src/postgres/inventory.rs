use crate::postgres::inspect::{
    AggregateInfo, ColumnInfo, ConstraintInfo, DomainInfo, EnumInfo, ExtensionInfo, FunctionInfo,
    GrantInfo, IndexInfo, MaterializedViewInfo, RlsPolicyInfo, SchemaInfo, SequenceInfo, TableInfo,
    TriggerInfo, ViewInfo,
};

#[derive(Debug, Clone)]
pub struct PostgresInventory {
    pub schemas: Vec<SchemaInfo>,
    pub tables: Vec<TableInfo>,
    pub columns: Vec<ColumnInfo>,
    pub extensions: Vec<ExtensionInfo>,
    pub enums: Vec<EnumInfo>,
    pub domains: Vec<DomainInfo>,
    pub sequences: Vec<SequenceInfo>,
    pub indexes: Vec<IndexInfo>,
    pub views: Vec<ViewInfo>,
    pub materialized_views: Vec<MaterializedViewInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub functions: Vec<FunctionInfo>,
    pub aggregates: Vec<AggregateInfo>,
    pub triggers: Vec<TriggerInfo>,
    pub grants: Vec<GrantInfo>,
    pub rls_policies: Vec<RlsPolicyInfo>,
}
