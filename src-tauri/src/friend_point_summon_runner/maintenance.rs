//! DRAFT: not declared by the parent module and therefore not compiled/tested.
//! Integration blockers and continuation steps:
//! docs/development/friend-point-inventory-maintenance-handoff.md
//! Intended to run under the summon runner's existing automation lease.
use super::*;
use crate::craft_essence_enhancement_runner::{
    CraftEssenceEnhancementMode, CraftEssenceEnhancementRunner, CraftEssenceEnhancementRunnerState,
    CycleBaseRarity,
};

const MS: &str = "InventoryMaintenance";
const CLOSE: Point = Point::new(0.0677, 0.0556);
const MENU: Point = Point::new(0.932, 0.913);
const BURN_COUNT: NormRect = NormRect {
    x: 0.632,
    y: 0.142,
    w: 0.079,
    h: 0.069,
};

fn safe_burn_candidate(c: &crate::screen::BurnServantCandidate) -> bool {
    (1..=3).contains(&c.rarity_max)
        && c.servant_label_score >= 0.94
        && c.lock_score.is_finite()
        && c.lock_score <= 0.5
        && c.region.x >= 0.055
        && c.region.x + c.region.w <= 0.81
        && c.region.y >= 0.251
        && c.region.y + c.region.h <= 1.0
}

impl FriendPointSummonRunner {
    fn maintenance_probe(&mut self, element: &str) -> Result<bool, String> {
        self.sidecar()
            .find_element_by_name(None, MS, element)
            .map(|m| m.found)
    }
    fn maintenance_tap(&mut self, point: Point) -> Result<(), String> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err("操作已取消".into());
        }
        if !self.tap_at(MS, point) {
            return Err("仓库维护点击失败".into());
        }
        thread::sleep(Duration::from_millis(650));
        Ok(())
    }
    fn maintenance_swipe(&mut self, from: Point, to: Point) -> Result<(), String> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err("操作已取消".into());
        }
        self.touch.swipe_with_settle(
            from.to_physical(self.screen_w, self.screen_h),
            to.to_physical(self.screen_w, self.screen_h),
            520,
            180,
        )?;
        thread::sleep(Duration::from_millis(600));
        Ok(())
    }
    fn maintenance_wait(&mut self, element: &str) -> Result<(), String> {
        let mut stable = 0;
        for _ in 0..30 {
            if self.cancel.load(Ordering::Relaxed) {
                return Err("操作已取消".into());
            }
            stable = if self.maintenance_probe(element)? {
                stable + 1
            } else {
                0
            };
            if stable >= 2 {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(400));
        }
        Err(format!("仓库维护页面未出现：{element}"))
    }
    fn burn_count(&mut self) -> Result<u32, String> {
        let text = self.sidecar().ocr_region(None, BURN_COUNT)?.full_text;
        crate::enhancement_runner::parse_selected_count(&text)
            .ok_or_else(|| format!("无法读取变还选择数量：{text}"))
    }
    fn wait_burn_count(&mut self, expected: u32) -> Result<(), String> {
        for _ in 0..12 {
            if self.maintenance_probe("burn_title")?
                && !self.maintenance_probe("burn_confirm")?
                && self.burn_count().ok() == Some(expected)
            {
                return Ok(());
            }
            if self.cancel.load(Ordering::Relaxed) {
                return Err("操作已取消".into());
            }
            thread::sleep(Duration::from_millis(500));
        }
        Err(format!("变还数量未变为 {expected}，停止后续操作"))
    }
    fn configure_burn_filter(&mut self) -> Result<(), String> {
        self.maintenance_tap(Point::new(0.766, 0.177))?;
        self.maintenance_wait("burn_filter")?;
        let thumb = self.sidecar().find_element_by_name(
            None,
            "CraftEssenceEnhancement",
            "scroll_bar_enhancement_filter",
        )?;
        if !thumb.found {
            return Err("未找到从者筛选滚动条".into());
        }
        self.maintenance_swipe(Point::new(thumb.x, thumb.y), Point::new(0.888, 0.115))?;
        self.maintenance_tap(Point::new(0.175, 0.884))?;
        for _ in 0..12 {
            let mut changed = false;
            for index in 0..5 {
                let region = NormRect {
                    x: 0.157 + f64::from(index) * 0.1465,
                    y: 0.315,
                    w: 0.025,
                    h: 0.032,
                };
                let luma = self.sidecar().read_region_luma(None, region)?;
                let on = if luma >= 180. {
                    true
                } else if luma <= 145. {
                    false
                } else {
                    return Err("从者星级筛选状态不明确".into());
                };
                if on != (index >= 2) {
                    self.maintenance_tap(Point::new(
                        region.x + region.w / 2.,
                        region.y + region.h / 2.,
                    ))?;
                    changed = true;
                    break;
                }
            }
            if !changed {
                self.configure_burn_kind()?;
                self.maintenance_tap(Point::new(0.824, 0.884))?;
                self.maintenance_wait("select_mode")?;
                return Ok(());
            }
        }
        Err("无法确认仅一至三星从者筛选".into())
    }

    fn configure_burn_kind(&mut self) -> Result<(), String> {
        for _ in 0..16 {
            let label = self
                .sidecar()
                .find_element_by_name(None, MS, "burn_filter_kind")?;
            if label.found && label.y < 0.65 {
                for _ in 0..6 {
                    let mut changed = false;
                    for index in 0..3 {
                        let region = NormRect {
                            x: 0.275 + f64::from(index) * 0.166,
                            y: label.y + 0.065,
                            w: 0.025,
                            h: 0.032,
                        };
                        let luma = self.sidecar().read_region_luma(None, region)?;
                        let on = if luma >= 180. {
                            true
                        } else if luma <= 145. {
                            false
                        } else {
                            return Err("种类筛选状态不明确".into());
                        };
                        if on != (index == 0) {
                            self.maintenance_tap(Point::new(
                                region.x + region.w / 2.,
                                region.y + region.h / 2.,
                            ))?;
                            changed = true;
                            break;
                        }
                    }
                    if !changed {
                        return Ok(());
                    }
                }
                return Err("无法确认仅从者种类筛选".into());
            }
            self.maintenance_swipe(Point::new(0.7, 0.75), Point::new(0.7, 0.50))?;
        }
        Err("未找到从者种类筛选，不执行变还".into())
    }

    fn burn_low_rarity_servants(&mut self) -> Result<(), String> {
        self.maintenance_wait("burn_title")?;
        if !self.maintenance_probe("servant_tab")? {
            self.maintenance_tap(Point::new(0.079, 0.17))?;
        }
        self.maintenance_wait("servant_tab")?;
        self.maintenance_wait("select_mode")?;
        self.wait_burn_count(0)?;
        self.configure_burn_filter()?;
        for attempt in 0..4 {
            if self
                .sidecar()
                .find_element_by_name(None, "CraftEssenceEnhancement", "button_scale_level_3")?
                .found
            {
                break;
            }
            if attempt == 3 {
                return Err("未确认从者列表为七列显示".into());
            }
            self.maintenance_tap(Point::new(0.024, 0.938))?;
        }
        let mut selected = 0;
        let mut empty_pages = 0;
        let mut previous: Option<HomeReference> = None;
        let mut unchanged = 0;
        for _ in 0..2000 {
            if self.cancel.load(Ordering::Relaxed) {
                return Err("操作已取消".into());
            }
            self.maintenance_wait("select_mode")?;
            self.wait_burn_count(selected)?;
            let candidates = self.sidecar().read_burn_servants()?.candidates;
            if let Some(card) = candidates.iter().find(|c| safe_burn_candidate(c)) {
                self.maintenance_tap(Point::new(
                    card.region.x + card.region.w / 2.,
                    card.region.y + card.region.h / 2.,
                ))?;
                selected += 1;
                self.wait_burn_count(selected)?;
                unchanged = 0;
                previous = None;
                if selected < 20 {
                    continue;
                }
            } else if selected == 0 {
                // At the end, a full-frame list-region comparison must confirm
                // that successive swipes no longer move the inventory.
                let current = capture_frame(self.sidecar())?;
                if let Some(old) = previous.as_ref() {
                    let matched = self.sidecar().find_region(
                        Some(current.frame.path()),
                        old.frame.path(),
                        NormRect {
                            x: 0.06,
                            y: 0.26,
                            w: 0.73,
                            h: 0.68,
                        },
                        NormRect {
                            x: 0.06,
                            y: 0.26,
                            w: 0.73,
                            h: 0.68,
                        },
                        0.995,
                        None,
                        None,
                    )?;
                    unchanged = if matched.found { unchanged + 1 } else { 0 };
                }
                previous = Some(current);
                if unchanged >= 2 {
                    self.emit(MS, "低星未锁定从者处理完成");
                    return Ok(());
                }
                empty_pages += 1;
                if empty_pages > 100 {
                    return Err("从者列表未能确认到底，停止维护".into());
                }
                self.maintenance_swipe(Point::new(0.7, 0.88), Point::new(0.7, 0.31))?;
                continue;
            }
            self.maintenance_tap(Point::new(0.901, 0.934))?;
            self.maintenance_wait("burn_confirm")?;
            // Only this batch's verified selections can authorize the final tap.
            if selected == 0 || selected > 20 {
                return Err("缺少合法变还批次记录".into());
            }
            self.maintenance_tap(Point::new(0.656, 0.871))?;
            self.wait_burn_count(0)?;
            self.emit(MS, &format!("已变还 {selected} 张低星未锁定从者"));
            selected = 0;
            empty_pages = 0;
            previous = None;
            unchanged = 0;
        }
        Err("从者变还次数超限".into())
    }

    fn perform_inventory_maintenance(&mut self) -> Result<(), String> {
        if self.home_reference.lock().unwrap().is_none() {
            return Err("没有记录本次友情池主页，不能执行自动维护".into());
        }
        self.emit(MS, "仓库已满，开始自动变还和循环搓丸子");
        self.maintenance_tap(Point::new(0.271, 0.662))?;
        self.burn_low_rarity_servants()?;
        self.maintenance_tap(CLOSE)?;
        self.maintenance_wait("shop_title")?;
        self.maintenance_tap(MENU)?;
        self.maintenance_wait("global_menu")?;
        self.maintenance_tap(Point::new(0.367, 0.84))?;
        self.maintenance_wait("ce_entry")?;
        let entry = self.sidecar().find_element_by_name(None, MS, "ce_entry")?;
        self.maintenance_tap(Point::new(entry.x, entry.y))?;
        // Share the live stream and cancellation token, keeping the parent lease.
        let sidecar = self.sidecar.take().ok_or("视频流不可用")?;
        let runner = CraftEssenceEnhancementRunner::new(
            self.adb.clone(),
            sidecar,
            self.app_handle.clone(),
            Arc::new(Mutex::new(CraftEssenceEnhancementRunnerState::Running)),
            self.cancel.clone(),
            (self.screen_w, self.screen_h),
            None,
            CraftEssenceEnhancementMode::Cycle,
        )
        .with_cycle_base_rarity(self.cycle_base_rarity);
        let (sidecar, result) = runner.run_embedded_cycle();
        self.sidecar = Some(sidecar);
        result?;
        if self.cancel.load(Ordering::Relaxed) {
            return Err("操作已取消".into());
        }
        // Finish may leave either the CE main page or its target list.
        for _ in 0..3 {
            if self.maintenance_probe("enhancement_title")? {
                break;
            }
            self.maintenance_tap(CLOSE)?;
        }
        self.maintenance_wait("enhancement_title")?;
        self.maintenance_tap(MENU)?;
        self.maintenance_wait("global_menu")?;
        self.maintenance_tap(Point::new(0.5, 0.84))?;
        let mut stable = 0;
        for _ in 0..40 {
            if self.cancel.load(Ordering::Relaxed) {
                return Err("操作已取消".into());
            }
            stable = if self.observe_start(false)? == ObservedScreen::Main {
                stable + 1
            } else {
                0
            };
            if stable >= 2 {
                self.emit(MS, "已确认返回本次记录的友情池主页，恢复召唤");
                return Ok(());
            }
            thread::sleep(POLL_INTERVAL);
        }
        Err("未返回本次记录的友情池主页，拒绝继续召唤".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn burn_rejects_high_rarity_uncertain_lock_and_outside_grid() {
        let mut c = crate::screen::BurnServantCandidate {
            region: NormRect {
                x: 0.1,
                y: 0.3,
                w: 0.1,
                h: 0.2,
            },
            rarity_max: 3,
            servant_label_score: 0.99,
            lock_score: 0.1,
        };
        assert!(safe_burn_candidate(&c));
        c.rarity_max = 4;
        assert!(!safe_burn_candidate(&c));
        c.rarity_max = 3;
        c.lock_score = 0.6;
        assert!(!safe_burn_candidate(&c));
        c.lock_score = 0.1;
        c.region.x = 0.9;
        assert!(!safe_burn_candidate(&c));
    }
}
