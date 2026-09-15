//! Rules for carrying accumulated CE experience into a fresh level-one base.
use super::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CycleBaseRarity {
    OneStar,
    TwoStar,
    #[default]
    Both,
}

impl CycleBaseRarity {
    pub(super) fn accepts(self, rarity: Option<u8>) -> bool {
        matches!(
            (self, rarity),
            (Self::OneStar | Self::Both, Some(1)) | (Self::TwoStar | Self::Both, Some(2))
        )
    }
}

pub(super) fn completed(cell: &CraftEssenceGridCell) -> bool {
    cell.valid && matches!(cell.rarity, Some(1 | 2)) && cell.level.is_some_and(|level| level >= 50)
}

pub(super) fn base(cell: &CraftEssenceGridCell, rarity: CycleBaseRarity) -> bool {
    cell.valid && !cell.locked && cell.level == Some(1) && rarity.accepts(cell.rarity)
}

pub(super) fn material(cell: &CraftEssenceGridCell) -> bool {
    cell.valid
        && !cell.locked
        && matches!(cell.rarity, Some(1 | 2))
        && cell.level.is_some_and(|level| (1..50).contains(&level))
}

pub(super) fn replacement(cell: &CraftEssenceGridCell) -> bool {
    material(cell) && !cell.selected && cell.level.is_some_and(|level| level > 1)
}

/// A fully audited selection must cover every sequence number exactly once.
/// Missing OCR is not evidence of an empty slot or a safe material.
pub(super) fn audit_selection(cells: &[CraftEssenceGridCell], count: u8) -> Result<(), String> {
    if !(1..=20).contains(&count) {
        return Err("自动选材数量无效".into());
    }
    let mut indices = HashSet::new();
    for cell in cells.iter().filter(|cell| cell.selected) {
        if !material(cell) {
            return Err("自动选材包含锁定、50 级以上或非一二星礼装".into());
        }
        let index = cell.selection_index.ok_or("无法识别所选素材序号")?;
        if index > count || !indices.insert(index) {
            return Err("素材序号重复或超出游戏计数".into());
        }
    }
    if indices.len() != usize::from(count) {
        return Err("尚未核对全部自动选择素材".into());
    }
    Ok(())
}

pub(super) fn may_replace_last(cell: &CraftEssenceGridCell, count: u8) -> bool {
    material(cell)
        && cell.selected
        && cell.selection_index == Some(count)
        && (!cell.same_as_target || count > 4)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn card(level: u32, rarity: u8) -> CraftEssenceGridCell {
        serde_json::from_value(serde_json::json!({
            "row":0,"col":0,"region":{"x":0.1,"y":0.3,"w":0.1,"h":0.2},
            "level":level,"rarity":rarity,"locked":false,"lockScore":0.0,"valid":true
        }))
        .unwrap()
    }
    #[test]
    fn configurable_level_one_bases_and_saved_products() {
        assert!(base(&card(1, 1), CycleBaseRarity::OneStar));
        assert!(!base(&card(1, 2), CycleBaseRarity::OneStar));
        assert!(base(&card(1, 2), CycleBaseRarity::TwoStar));
        assert!(!base(&card(2, 2), CycleBaseRarity::Both));
        for level in [50, 51, 55] {
            assert!(completed(&card(level, 2)));
            assert!(!material(&card(level, 2)));
        }
        assert!(replacement(&card(49, 2)));
        assert!(!replacement(&card(1, 2)));
        let mut locked = card(20, 1);
        locked.locked = true;
        assert!(!replacement(&locked));
    }
    #[test]
    fn audit_accepts_game_cap_before_twenty_and_rejects_unknown_or_unsafe() {
        let mut cells: Vec<_> = (1..=7)
            .map(|i| {
                let mut c = card(1, 1);
                c.selected = true;
                c.selection_index = Some(i);
                c
            })
            .collect();
        assert!(audit_selection(&cells, 7).is_ok());
        assert!(audit_selection(&cells, 20).is_err());
        cells[6].level = Some(50);
        assert!(audit_selection(&cells, 7).is_err());
        cells[6].level = Some(1);
        cells[6].selection_index = None;
        assert!(audit_selection(&cells, 7).is_err());
    }
    #[test]
    fn preserve_limit_break_copies_and_use_actual_last_index() {
        let mut c = card(1, 2);
        c.selected = true;
        c.same_as_target = true;
        c.selection_index = Some(4);
        assert!(!may_replace_last(&c, 4));
        c.selection_index = Some(19);
        assert!(may_replace_last(&c, 19));
        assert!(!may_replace_last(&c, 20));
    }
}
