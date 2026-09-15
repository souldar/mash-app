# 友情池满仓维护：跨电脑开发交接

更新：2026-09-15。本文记录正在开发的分支快照，不代表功能已交付。

## 从这里接续

- 仓库：`git@github.com:souldar/mash-app.git`
- 分支：`codex/friend-point-inventory-maintenance`
- 基点：`950cde364b954672c3f0c57439d1a6d4ea4cf5ef`（原友情点召唤分支）。
- 目标：友情池召唤满仓 → 低星从者变还 → 循环搓丸子 → 验证返回原卡池 → 恢复召唤。
- 当前最关键事实：`src-tauri/src/friend_point_summon_runner/maintenance.rs` **没有被父模块声明，不参与编译或测试**。它是需要继续修正和接线的草稿；全套测试通过也不代表该文件可编译。
- 尚未实际执行任何自动变还、自动强化或恢复召唤；现场仅做了导航、截图、可撤销的选材和取消确认。
- 本分支用于接续开发。原 PR `https://github.com/ZaiZheTingDun/mash-app/pull/1` 不因本次交接更新；未创建新 PR，也未发布版本。

给另一台电脑 Codex 的起始消息：

> 请读取 AGENTS.md 和 docs/development/friend-point-inventory-maintenance-handoff.md，在 codex/friend-point-inventory-maintenance 分支继续完成满仓自动变还与循环搓丸子。先处理文档列出的编译和流程接线缺口，再验证恢复原友情池及计数。保留普通召唤行为和临时模板生命周期，不把未接入模块误认为已验证。

## 获取代码与环境

全新目录：

```bash
git clone --branch codex/friend-point-inventory-maintenance git@github.com:souldar/mash-app.git
cd mash-app
git remote add upstream git@github.com:ZaiZheTingDun/mash-app.git
```

已有该 fork 的干净工作目录：

```bash
git fetch origin
git switch --track origin/codex/friend-point-inventory-maintenance
```

如果本地分支已存在，切换后使用 `git pull --ff-only`；不要覆盖其他未提交工作。查看 `git log -1` 确认取得最新交接提交。

准备 Node、项目 `package.json` 指定的 pnpm 10.32.0、稳定版 Rust、平台 Tauri 编译依赖、Python 3.11–3.14 和 Poetry。然后在仓库根目录执行：

```bash
pnpm install --frozen-lockfile
poetry -C sidecar/mash_cv install
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
poetry -C sidecar/mash_cv run pytest
pnpm tauri dev
```

Vite 使用 1420 端口，启动前检查是否已有开发进程。CV 重型运行时需在应用里按提示下载安装；debug 模式 `resolve_sidecar_code_dir` 优先使用仓库 `sidecar/mash_cv`，模板与 CV 配置也优先读取仓库资源，因此不要用旧安装包来验证这些 Python 改动。仅 `poetry install` 不会替应用安装 PyInstaller 运行时。详见 [README](../../README.md#cv-runtime)。

另一台电脑需要自行配置 GitHub SSH/gh 登录、模拟器、ADB 调试和系统权限，认证信息不随 Git 迁移。原设备为 CN 客户端 BlueStacks、1920×1080，ADB 地址 `127.0.0.1:5555`；这是现场信息，不应写死在新环境。原 scrcpy push 失败是模拟器未允许 ADB 调试，用户已解决。

`.screenshots/`、`/tmp`、Python 虚拟环境、Rust target、node_modules、应用数据与运行中的视频流均不迁移。用于回归的截图和模板已放入仓库；测试不得依赖上述临时路径。友情池临时模板属于进程内状态，换机必须重新手动开始以记录本次卡池。

## 已确认的业务约定

1. 从首页进入变还：菜单 → 商店 → 灵基变还 → 从者。满仓弹窗的变还入口也有现场截图。
2. 仅变还三星及以下、明确未锁定的从者；不变还经验值卡、芙芙、礼装和指令纹章，也不自动解锁。
3. 筛选同时限制星级和种类。种类在滚动内容中：仅“从者”开启，“经验值”“芙芙”关闭；白色是开启，蓝色是关闭。定位标题后相对采样，不能假定固定滚动距离。
4. 礼装底卡每轮使用新的 1 级卡，星级可选一星、二星或两者。游戏自动选材设置一星、二星、未强化、已强化；其余星级关闭。
5. 根据用户观察，游戏优先选择未强化同名卡、已强化同名卡、未强化非同名卡。自动选材可能因预估经验达到上限而不足 20 张；以实际计数为准。
6. 有合适的未锁定、2–49 级一二星素材时，尝试替换最后一张自动素材，再强化；没有时直接使用原自动选择结果。需要保留用于突破的同名素材，不能盲目移除。
7. 本轮强化结果未到 50 级，留作后续素材，下一轮换新的 1 级底卡；达到 50 级或以上就是成品，锁定保存。已有未锁定的 ≥50 级成品也要先保护，不能让自动选材吃掉。
8. 原友情池临时模板一直保留到下一次**手动开始**。确认弹窗消失后的短暂主页画面不应停止正常召唤。仅维护结束返回时用原模板核对身份，不重新捕获来“证明”返回成功。
9. 无法确认类型、锁定、素材序号或原卡池时停止并报告原因。取消操作不能继续点击确认，且最终状态应为 Idle。

## 代码和证据地图

| 位置 | 当前内容与验证边界 |
| --- | --- |
| `src-tauri/src/craft_essence_enhancement_runner/cycle_policy.rs` | 底卡、素材、成品、选择审核及末位替换规则，有单元测试 |
| `src-tauri/src/craft_essence_enhancement_runner/cycle.rs` | 循环状态机，已挂入 CE runner；可编译不等于经过实机验证 |
| `src-tauri/src/craft_essence_enhancement_runner/{policy,dialogs}.rs` | Cycle 推荐素材配置；增强素材开启，自动重新配置关闭，逐轮主动执行 |
| `src-tauri/src/commands/automation/craft_essence.rs` | Cycle 模式及可选 `baseRarity` 参数 |
| `src/features/craft-essence-enhancement/` | 循环策略按钮和底卡星级 UI、调用参数测试 |
| `src-tauri/src/friend_point_summon_runner/maintenance.rs` | 未声明的维护编排草稿：筛选、候选变还、页面导航、嵌入 CE 与原卡池返回 |
| `src-tauri/src/screen/{protocol,types}.rs`、`screen/operations/enhancement.rs` | `read_burn_servants` IPC；CE grid 增加 selected / selectionIndex |
| `sidecar/mash_cv/mash_cv/cv.py` | 低星从者候选、绿色选中卡片网格恢复和序号、二星等级上限 OCR |
| `src-tauri/resources/servers/cn/cv.json` | 满仓、推荐选材警告和 InventoryMaintenance 页面/元素探针，仅 CN 新增 |
| `src-tauri/resources/servers/{cn,shared}/templates/` | CN 导航/变还模板，共享素材序号 1–20 模板 |
| `sidecar/mash_cv/tests/test_data/screenshots/` | `friend_point_summon/ce_inventory_full.png`、`enhancement_ce/cycle_*.png`、`inventory_maintenance/`；均为离线测试输入 |

素材序号 1–19 来自现场截图；序号 20 使用同字体数字组合，仍需真实截图验证。1280 宽度下部分序号会返回未知，测试要求不得错认成另一序号，不意味着该分辨率全流程可用。

现场最后停在变还筛选的“种类”区域，只有从者开启，尚未点击“决定”；不要假定新电脑或新会话仍停在相同位置。变还页左侧“推荐”是标记/收藏相关模式，**不是**自动选材，不要误用。

## 下一步：按依赖顺序处理

### 1. 接入并编译维护模块

先查看父 runner 的字段和现有生命周期，再声明 `mod maintenance;`。草稿有这些已知缺口：

- 父 runner 尚无草稿所用的 `adb` 和 `cycle_base_rarity` 字段，需要补齐参数传递或调整结构；友情池 UI/命令尚无维护配置接线。
- `CraftEssenceEnhancementRunner::run_embedded_cycle` 尚不存在。需要让嵌入执行返回 sidecar 和 Result，共用父自动化租约、取消令牌和视频流；检查 CE runner 的 Drop，不能意外关闭或缓存父视频流。
- 草稿列表到底检测对 `find_region` 的调用签名与当前 API 不符，且旧接口返回 `Option<Point>`；应使用正确的裁剪模板比较接口并测试。
- 模块内测试目前也未运行。接入后先修复编译，再新增编排测试。

### 2. 接通满仓与恢复召唤

- 满仓弹窗优先级高于背景 Continue/Skip/Home 探针，稳定确认后才切换维护。
- 必须已有本次主页记录；不能从未验证的启动画面直接进行清仓。
- 区分 AwaitResult 与 AwaitConfirmationRepeat 等阶段，维护前后同一召唤批次只能记账一次。
- 维护完成后用保留的主页模板确认原卡池，然后进入正常百抽确认流程，不能走会重新采样主页的启动路径。
- 同一召唤进度不得无限重复清仓。菜单“召唤”可能打开不同卡池，未匹配原模板时不得继续。
- 取消和异常必须归还流所有权，释放租约，保留正确生命周期。

### 3. 审核仍未验证的边界

- 变还筛选设置、选择数量逐次核对、最终确认及变还后的奖励/额外警告页面。草稿只覆盖已采集的确认框，未验证三星额外提示或实际结果页。
- 礼装滚动到顶需验证成功；草稿只读取滚动条后拖动。
- CE 素材跨页聚合使用 BTreeMap，重复序号证据可能被覆盖，不能让错误识别绕过审核。
- `same_card_position` 使用图像指纹；选中后绿色覆盖可能改变指纹，需验证替换后的身份检查。
- `may_replace_last` 的“总数 > 4”不能单独证明保留四张同名卡，补充同名数量/顺序测试。
- 取消期间若操作返回 Err，Cycle 当前可能被标为 Error，应与正常取消统一。
- 维护结束后最多三次 Close 的草稿导航应改成明确页面守卫，不能在未知弹窗盲点。
- 验证无素材、无可用底卡、锁定失败、底卡/成品等级 OCR 不明确时的结束行为。

### 4. 完成验证再交付

先运行下面三层检查，再在设备上分段验证。变还/强化会实际消耗卡片，执行前明确现场状态与选择内容。最后整理提交和 PR；当前仅推送开发快照。

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
pnpm build
poetry -C sidecar/mash_cv run pytest
```

## 交接时验证记录

本次推送前重新执行：

- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`：通过；未声明的维护草稿额外执行了 rustfmt。
- `cargo test --manifest-path src-tauri/Cargo.toml`：552 passed，0 failed。首次发现新增模板目录缺少 Tauri bundle glob，已补齐并重跑通过。
- `pnpm build`：ESLint、TypeScript、37 个测试文件 / 374 个测试及 Vite 构建通过；仍有 Vite 大 chunk 提示。
- `poetry -C sidecar/mash_cv run pytest -q`：492 passed，5 skipped。
- `git diff --check`：通过。

这些结果不包含未声明的维护模块，也不代表实际设备上的消耗操作、导航或恢复召唤已经验证。

相关设计：[友情池状态机](../state-machines/friend-point-summon-automation.md)、[礼装强化状态机](../state-machines/craft-essence-enhancement-automation.md)。
