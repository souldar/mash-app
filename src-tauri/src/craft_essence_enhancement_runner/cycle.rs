//! One-shot automatic material selection followed by an audited last-slot swap.
use super::*;
use cycle_policy::{self as rules, CycleBaseRarity};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Protect,
    Base,
    Auto,
    Audit,
    Remove,
    Replace,
    VerifyReplacement,
    Commit,
    Enhance,
    Confirm,
    Return,
    Lock,
    VerifyLock,
}

impl CraftEssenceEnhancementRunner {
    pub(super) fn run_cycle(&mut self, rarity: CycleBaseRarity) {
        self.transition(LifecycleEvent::WorkerStarted);
        self.emit("", "循环搓丸子：先保护成品，再选择 1 级底卡");
        match self.cycle_loop(rarity) {
            Ok(()) if self.cancel.load(Ordering::Relaxed) => {
                self.transition(LifecycleEvent::StopRequested)
            }
            Ok(()) => {
                self.transition(LifecycleEvent::Finished);
                self.emit("", "循环搓丸子结束，已保存完成的丸子");
            }
            Err(message) => self.fail("Cycle", message),
        }
    }

    fn cycle_tap(&mut self, point: Point) -> Result<(), String> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err("操作已取消".into());
        }
        if !self.tap_at("Cycle", point) {
            return Err("点击失败，循环搓丸子停止".into());
        }
        Ok(())
    }

    fn cycle_scroll(&mut self, top: bool) -> Result<(), String> {
        let (from, to) = if top {
            // The thumb is read before dragging; never guess its current position.
            let grid = self.cycle_grid()?;
            let y = grid
                .diagnostics
                .scrollbar_thumb_y
                .ok_or("未识别到列表滚动条")?;
            (
                Point::new(LIST_SCROLLBAR_X, y),
                Point::new(LIST_SCROLLBAR_X, LIST_SCROLLBAR_OVERSHOOT_Y),
            )
        } else {
            (LIST_SWIPE_FROM, LIST_SWIPE_TO)
        };
        if !self.swipe_at("Cycle", from, to, 520) {
            return Err("滚动礼装列表失败".into());
        }
        thread::sleep(Duration::from_millis(650));
        Ok(())
    }

    fn cycle_grid(&mut self) -> Result<crate::screen::ReadCraftEssenceGridResult, String> {
        self.sidecar().read_craft_essence_grid(
            None,
            "enhancement_ce/item_ce_bar_bronze",
            1920.0,
            ITEM_GRID_REGION,
            1.2,
        )
    }

    fn cycle_loop(&mut self, rarity: CycleBaseRarity) -> Result<(), String> {
        let mut phase = Phase::Protect;
        let mut pages = 0;
        let mut waits = 0;
        let mut reset_top = true;
        let mut lock_card: Option<CraftEssenceGridCell> = None;
        let mut selected = BTreeMap::<u8, CraftEssenceGridCell>::new();
        let mut count = 0;
        let mut replacement_exists = false;
        let mut replacement_card: Option<CraftEssenceGridCell> = None;
        let mut left_main = false;
        let mut returned_checks = 0;
        let mut recommendation_confirmed = false;
        loop {
            if self.cancel.load(Ordering::Relaxed) {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(500));
            waits += 1;
            if waits > 40 {
                return Err(format!("循环搓丸子等待页面超时：{phase:?}"));
            }
            if phase == Phase::Auto
                && self.probe("dialog_enhancement_ce_recommend_selection_confirm")
            {
                if !self.recommend_execute_tapped
                    || self.recommend_configured_profile != Some(RecommendMaterialProfile::Cycle)
                {
                    return Err("推荐配置尚未核对，拒绝确认".into());
                }
                if !recommendation_confirmed {
                    self.cycle_tap(Point::new(0.656, 0.782))?;
                    recommendation_confirmed = true;
                }
                continue;
            }
            let screen = self.detect_screen();
            if screen == Screen::Unknown {
                continue;
            }
            if matches!(
                phase,
                Phase::Protect | Phase::Base | Phase::Audit | Phase::Remove | Phase::Replace
            ) {
                if screen == Screen::FilterDialog {
                    if !self.handle_filter_dialog() {
                        return Err("配置礼装筛选失败".into());
                    }
                    continue;
                }
                if screen == Screen::OrderDialog {
                    if !self.handle_order_dialog() {
                        return Err("配置礼装排序失败".into());
                    }
                    continue;
                }
            }
            match phase {
                Phase::Protect | Phase::Base => {
                    match screen {
                        Screen::Main {
                            target_selected, ..
                        } => {
                            self.cycle_tap(if target_selected {
                                TARGET_RESELECT_BUTTON
                            } else {
                                TARGET_SELECT_BUTTON
                            })?;
                            continue;
                        }
                        Screen::CraftEssenceSelect { descending } => {
                            if !self.density_checked {
                                if !self.ensure_max_density() {
                                    return Err("无法确认礼装列表显示密度".into());
                                }
                                self.density_checked = true;
                                continue;
                            }
                            if self.filter_two_star_enabled != Some(true) {
                                self.filter_two_star_desired = true;
                                self.filter_configured = false;
                                self.cycle_tap(FILTER_BUTTON)?;
                                continue;
                            }
                            if !self.order_configured {
                                self.cycle_tap(ORDER_BUTTON)?;
                                continue;
                            }
                            if descending != (phase == Phase::Protect) {
                                self.cycle_tap(ORDER_DIRECTION_BUTTON)?;
                                reset_top = true;
                                continue;
                            }
                        }
                        _ => continue,
                    }
                    if reset_top {
                        self.cycle_scroll(true)?;
                        reset_top = false;
                        waits = 0;
                        continue;
                    }
                    let grid = self.cycle_grid()?;
                    if !grid.found || grid.cells.iter().any(|c| !c.valid) {
                        continue;
                    }
                    let candidate = grid.cells.iter().find(|c| {
                        if phase == Phase::Protect {
                            rules::completed(c) && !c.locked
                        } else {
                            rules::base(c, rarity)
                        }
                    });
                    if let Some(card) = candidate {
                        if phase == Phase::Protect {
                            lock_card = Some(card.clone());
                            self.cycle_tap(UNIFIED_LOCK_BUTTON)?;
                            phase = Phase::Lock;
                        } else {
                            self.current_bomb_fingerprint = card.art_fingerprint.clone();
                            self.cycle_tap(center(card))?;
                            self.recommend_profile = RecommendMaterialProfile::Cycle;
                            self.recommend_configured_profile = None;
                            self.recommend_executed_for_target = false;
                            self.recommend_execute_tapped = false;
                            self.recommend_reset_done = false;
                            self.recommend_ready_waits = 0;
                            recommendation_confirmed = false;
                            self.strategy_stage = StrategyStage::FastAutoFeedPending;
                            phase = Phase::Auto;
                            self.emit("Cycle", "已选择 1 级底卡，配置一二星未强化及已强化素材");
                        }
                        waits = 0;
                        pages = 0;
                        continue;
                    }
                    if self.probe("element_enhancement_ce_scroll_end") {
                        if phase == Phase::Base {
                            return Ok(());
                        }
                        phase = Phase::Base;
                        reset_top = true;
                        pages = 0;
                        waits = 0;
                        self.emit("Cycle", "已核对库存成品锁定状态，开始寻找新底卡");
                        continue;
                    }
                    pages += 1;
                    if pages > 80 {
                        return Err("未确认礼装列表末尾，拒绝使用未核对的库存".into());
                    }
                    self.cycle_scroll(false)?;
                    waits = 0;
                }
                Phase::Lock => {
                    if screen != Screen::CraftEssenceLockMode {
                        continue;
                    }
                    let expected = lock_card.as_ref().ok_or("缺少锁定对象")?;
                    let grid = self.cycle_grid()?;
                    let current = grid.cells.iter().find(|c| same_card_position(c, expected));
                    if !current.is_some_and(|c| rules::completed(c) && !c.locked) {
                        return Err("锁定前卡片位置或状态发生变化".into());
                    }
                    self.cycle_tap(center(expected))?;
                    phase = Phase::VerifyLock;
                    waits = 0;
                }
                Phase::VerifyLock => {
                    if screen != Screen::CraftEssenceLockMode {
                        continue;
                    }
                    let expected = lock_card.as_ref().ok_or("缺少锁定对象")?;
                    let grid = self.cycle_grid()?;
                    if grid
                        .cells
                        .iter()
                        .any(|c| same_card_position(c, expected) && rules::completed(c) && c.locked)
                    {
                        self.cycle_tap(SELECT_OBJECT_BUTTON)?;
                        phase = Phase::Protect;
                        reset_top = true;
                        pages = 0;
                        waits = 0;
                        self.emit("Cycle", "已确认成品锁定，继续核对库存");
                    }
                    // Never toggle twice when the lock icon cannot be verified.
                }
                Phase::Auto => match screen {
                    Screen::Main {
                        target_selected: true,
                        ..
                    } => {
                        if self.recommend_executed_for_target {
                            self.cycle_tap(MATERIAL_SELECT_BUTTON)?;
                            phase = Phase::Audit;
                            selected.clear();
                            count = 0;
                            replacement_exists = false;
                            self.density_checked = false;
                            self.order_configured = false;
                            self.order_level_selected = false;
                            reset_top = true;
                            pages = 0;
                            waits = 0;
                        } else {
                            self.cycle_tap(RECOMMEND_MATERIAL_BUTTON)?;
                        }
                    }
                    Screen::RecommendMaterialDialog => {
                        if !self.handle_recommend_material_dialog() {
                            return Err("配置推荐素材失败".into());
                        }
                    }
                    Screen::RecommendMaterialEmptyDialog => {
                        self.cycle_tap(RECOMMEND_EMPTY_CLOSE_BUTTON)?;
                        return Ok(());
                    }
                    _ => {}
                },
                Phase::Audit | Phase::Remove | Phase::Replace => {
                    if screen != Screen::MaterialSelect {
                        continue;
                    }
                    if !self.density_checked {
                        if !self.ensure_max_density() {
                            return Err("无法确认素材列表显示密度".into());
                        }
                        self.density_checked = true;
                        continue;
                    }
                    if !self.order_configured {
                        self.cycle_tap(ORDER_BUTTON)?;
                        continue;
                    }
                    if reset_top {
                        self.cycle_scroll(true)?;
                        reset_top = false;
                        waits = 0;
                        continue;
                    }
                    let displayed = self.read_material_selected_count()?;
                    if phase == Phase::Audit && count == 0 {
                        count = displayed;
                    }
                    let expected = if phase == Phase::Replace {
                        count.saturating_sub(1)
                    } else {
                        count
                    };
                    if displayed != expected {
                        continue;
                    }
                    let grid = self.cycle_grid()?;
                    if !grid.found {
                        continue;
                    }
                    match phase {
                        Phase::Audit => {
                            for c in &grid.cells {
                                if c.selected {
                                    if !rules::material(c) {
                                        return Err("自动选材包含不允许消耗的礼装".into());
                                    }
                                    let index = c
                                        .selection_index
                                        .ok_or("所选素材序号识别不清，停止强化")?;
                                    if index > count {
                                        return Err("素材序号超出游戏计数".into());
                                    }
                                    selected.insert(index, c.clone());
                                }
                                replacement_exists |= rules::replacement(c);
                            }
                        }
                        Phase::Remove => {
                            if let Some(c) =
                                grid.cells.iter().find(|c| c.selection_index == Some(count))
                            {
                                if !rules::may_replace_last(c, count) {
                                    phase = Phase::Commit;
                                    waits = 0;
                                    continue;
                                }
                                self.cycle_tap(center(c))?;
                                phase = Phase::Replace;
                                reset_top = true;
                                pages = 0;
                                waits = 0;
                                continue;
                            }
                        }
                        Phase::Replace => {
                            if let Some(c) = grid.cells.iter().find(|c| rules::replacement(c)) {
                                replacement_card = Some(c.clone());
                                self.cycle_tap(center(c))?;
                                phase = Phase::VerifyReplacement;
                                waits = 0;
                                continue;
                            }
                        }
                        _ => unreachable!(),
                    }
                    let bottom = self.probe("element_enhancement_ce_scroll_end");
                    if phase == Phase::Audit
                        && ((selected.len() == usize::from(count) && replacement_exists) || bottom)
                    {
                        rules::audit_selection(
                            &selected.values().cloned().collect::<Vec<_>>(),
                            count,
                        )?;
                        if replacement_exists
                            && selected
                                .get(&count)
                                .is_some_and(|c| rules::may_replace_last(c, count))
                        {
                            phase = Phase::Remove;
                            reset_top = true;
                            pages = 0;
                        } else {
                            self.emit("Cycle", "没有可替换素材，保留游戏自动选择结果");
                            phase = Phase::Commit;
                        }
                        waits = 0;
                        continue;
                    }
                    if bottom {
                        return Err("未能重新找到最后素材或替换素材，停止本次强化".into());
                    }
                    pages += 1;
                    if pages > 80 {
                        return Err("素材列表扫描超限，停止本次强化".into());
                    }
                    self.cycle_scroll(false)?;
                    waits = 0;
                }
                Phase::VerifyReplacement => {
                    if screen != Screen::MaterialSelect {
                        continue;
                    }
                    if self.read_material_selected_count()? != count {
                        continue;
                    }
                    let expected = replacement_card.as_ref().ok_or("缺少替换素材记录")?;
                    let grid = self.cycle_grid()?;
                    if grid.cells.iter().any(|c| {
                        same_card_position(c, expected)
                            && rules::material(c)
                            && c.selected
                            && c.selection_index == Some(count)
                    }) {
                        phase = Phase::Commit;
                        waits = 0;
                    }
                }
                Phase::Commit => {
                    if screen != Screen::MaterialSelect {
                        continue;
                    }
                    if self.read_material_selected_count()? != count {
                        return Err("提交前素材数量变化".into());
                    }
                    self.cycle_tap(MATERIAL_DECIDE_BUTTON)?;
                    phase = Phase::Enhance;
                    waits = 0;
                }
                Phase::Enhance => {
                    if let Screen::Main {
                        target_selected: true,
                        ready: true,
                    } = screen
                    {
                        self.cycle_tap(ENHANCE_BUTTON)?;
                        phase = Phase::Confirm;
                        waits = 0;
                    }
                }
                Phase::Confirm => match screen {
                    Screen::EnhancedMaterialWarningDialog => {
                        if !self.swipe_at(
                            "Cycle",
                            ENHANCED_MATERIAL_WARNING_SLIDER_FROM,
                            ENHANCED_MATERIAL_WARNING_SLIDER_TO,
                            900,
                        ) {
                            return Err("确认素材滑动失败".into());
                        }
                        self.cycle_tap(ENHANCED_MATERIAL_WARNING_DECIDE_BUTTON)?;
                    }
                    Screen::EnhancementConfirmDialog => {
                        self.cycle_tap(ENHANCE_CONFIRM_BUTTON)?;
                        phase = Phase::Return;
                        left_main = false;
                        returned_checks = 0;
                        waits = 0;
                    }
                    _ => {}
                },
                Phase::Return => match enhancement_return_action(screen, left_main) {
                    EnhancementReturnAction::ObserveReturnedMain => {
                        returned_checks += 1;
                        if returned_checks < 2 {
                            continue;
                        }
                        let result = self.sidecar().read_craft_essence_main_target(None)?;
                        if !result.found {
                            continue;
                        }
                        let post_level = result.level.ok_or("强化后无法确认等级")?;
                        if !matches!(
                            result.level_cap,
                            Some(10 | 15 | 20 | 25 | 30 | 35 | 40 | 45 | 50 | 55)
                        ) {
                            return Err("强化后等级上限异常".into());
                        }
                        self.completed_enhancements += 1;
                        self.emit(
                            "Cycle",
                            &format!(
                                "本次产物 {post_level} 级，{}",
                                if post_level >= 50 {
                                    "返回列表锁定保存"
                                } else {
                                    "保留为素材，重新选择 1 级底卡"
                                }
                            ),
                        );
                        phase = Phase::Protect;
                        reset_top = true;
                        pages = 0;
                        waits = 0;
                        self.filter_two_star_enabled = None;
                        self.filter_configured = false;
                    }
                    EnhancementReturnAction::WaitForConfirmationClose => {}
                    EnhancementReturnAction::CloseExpOverflow => {
                        left_main = true;
                        self.cycle_tap(EXP_OVERFLOW_CLOSE_BUTTON)?;
                    }
                    EnhancementReturnAction::TapSkip { mark_left_main } => {
                        left_main |= mark_left_main;
                        returned_checks = 0;
                        self.cycle_tap(ENHANCEMENT_SKIP_BUTTON)?;
                    }
                    EnhancementReturnAction::Unexpected => {
                        return Err("强化期间进入非预期页面".into())
                    }
                },
            }
        }
    }
}

fn center(c: &CraftEssenceGridCell) -> Point {
    Point::new(c.region.x + c.region.w / 2.0, c.region.y + c.region.h / 2.0)
}
fn same_card_position(c: &CraftEssenceGridCell, expected: &CraftEssenceGridCell) -> bool {
    c.valid
        && c.level == expected.level
        && c.rarity == expected.rarity
        && (c.region.x - expected.region.x).abs() < 0.01
        && (c.region.y - expected.region.y).abs() < 0.02
        && same_art_fingerprint(&c.art_fingerprint, &expected.art_fingerprint)
}
