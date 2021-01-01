#[derive(Hash, PartialEq, Eq)]
pub enum Flag {
    CursorMoved,
    FullRefresh,
    DisplayRefresh,
    SetupDisplays,
    RemapActiveRows,
    UpdateOffset,
    DisplayCMDLine,
    ForceRerow,
    UpdateRow(usize)
}
