use crate::postgres::inspect::{
    ColumnInfo, ConstraintInfo, EnumInfo, ExtensionInfo, FunctionInfo, IndexInfo, SchemaInfo,
    SequenceInfo, TableInfo, ViewInfo,
};

#[derive(Debug, Clone)]
pub struct PostgresInventory {
    pub schemas: Vec<SchemaInfo>,
    pub tables: Vec<TableInfo>,
    pub columns: Vec<ColumnInfo>,
    pub extensions: Vec<ExtensionInfo>,
    pub enums: Vec<EnumInfo>,
    pub sequences: Vec<SequenceInfo>,
    pub indexes: Vec<IndexInfo>,
    pub views: Vec<ViewInfo>,
    pub constraints: Vec<ConstraintInfo>,
    pub functions: Vec<FunctionInfo>,
}
