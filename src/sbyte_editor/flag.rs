#[derive(Hash, PartialEq, Eq)]
pub enum Flag {
    CursorMoved,
    FullRefresh,
    DisplayRefresh,
    SetupDisplays,
    RemapActiveRows,
    UpdateOffset,
    DisplayCMDLine,
    HideFeedback,
    ForceRerow,
    UpdateRow(usize)
}
