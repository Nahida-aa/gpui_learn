# Zed Agent Slash 命令体系调研

> 来源仓库：`/home/aa/repos/learn_ls/zed`
> 分支/版本：`main`，HEAD `027cf0def7`（提交日期 2026-08-07）
> 调研目的：梳理 Zed agent 的 `/` 命令体系，重点确认 external-agents（外部 agent）是否由接入方决定。

## 重要前提：slash 命令体系已大改

此前的 `/file`、`/terminal`、`/now`、`/fetch`、`/docs` 等命令体系已在当前版本被整体删除：

```
76c6004b27 Remove text thread and slash command crates (#52757)
```

`crates/assistant_slash_command`（定义 `SlashCommand` trait、`SlashCommandWorkingSet`、`SlashCommandRegistry`）与 `crates/assistant_slash_commands`（`/file`、`/tab`、`/now`、`/fetch`、`/docs`、`/diagnostics`、`/prompt`、`/symbols`、`/terminal`、`/selection` 等实现）这两个 crate 已不存在。

旧体系能力并未消失，而是**迁移到了 `@` mention 体系**（如 `@file`、`@symbol`、`@fetch`、`@diagnostics`、`@selection`）。

## 1. 当前 slash 命令的定义位置

不再有集中的静态注册表。命令由 agent 通过 **ACP（Agent Client Protocol）** 在运行时上报。分三层：

- **UI/补全层** — `crates/agent_ui/src/completion_provider.rs`
  - `PromptCompletionProviderDelegate` trait（`completion_provider.rs:451-476`）定义候选来源：`available_commands`、`available_skills`、`available_local_commands`
  - `/` 语法解析：`SlashCommandCompletion::try_parse`（`completion_provider.rs:2016-2022`）
  - 候选类型 `SlashCompletionCandidate::{Skill, Command, LocalCommand}`（`completion_provider.rs:410-425`）
- **协议层** — `crates/acp_thread/src/acp_thread.rs`
  - agent 进程推送 `SessionUpdate::AvailableCommandsUpdate` → 存入 `available_commands`（`acp_thread.rs:2625-2631`）
  - 命令分类通过 ACP `meta` 字段携带：`COMMAND_CATEGORY_META_KEY`、`CommandCategory::{Native, Mcp}`（`acp_thread.rs:81-122`）
- **原生 agent 生产层** — `crates/agent/src/agent.rs:1503-1560`

## 2. 内置 slash 命令清单（当前版本）

| 类别 | 命令 | 作用 | 来源 |
|---|---|---|---|
| Native（硬编码，仅 1 个） | `/compact` | 压缩对话历史以释放上下文 | `crates/agent/src/agent.rs:170`（常量）、`1509-1515`（构造） |
| LocalCommand（纯 UI，2 个） | `/helpful`、`/not-helpful` | 会话反馈点赞/点踩，不发给模型 | `completion_provider.rs:199-209`；处理 `thread_view.rs:1158-1166` |
| Mcp（动态） | 来自 MCP server 的 prompts | 数量不固定，命名冲突时强制前缀 `/<server>.<name>` | `agent.rs:1528-1542` |
| Skill（动态） | 用户 `.md` 技能文件 | — | `agent.rs:2105` |
| External ACP（动态） | 外部 agent 上报的任意命令 | 无 category meta | 文档示例：Claude Agent 的 `/login`（`docs/src/ai/external-agents.md:45`） |

命名冲突处理保证裸 `/compact` 永远路由到原生命令（`agent.rs:1519-1536`）。

`@` mention 体系支持类型（`message_editor.rs:95-109`）：`File`、`Symbol`、`Thread`、`Diagnostics`、`Fetch`、`Skill`、`BranchDiff`。

## 3. External Agents（重点）：是否由接入方决定

**结论：存在，且是核心架构（基于 ACP）；是否可用、有哪些 external agent，完全由接入方（用户/配置者）决定，Zed 不硬编码白名单。**

核心文件：
- `docs/src/ai/external-agents.md`（外部 agent 文档）
- `crates/project/src/agent_server_store.rs`（external agent 存储，2340 行）
- `crates/project/src/agent_registry_store.rs`（注册表，677 行）
- `crates/agent_servers/src/custom.rs`（自定义接入，431 行）
- `crates/settings_ui/src/pages/external_agents_page.rs`（设置 UI，1166 行）

### 三条接入路径（全部用户侧控制）

1. **ACP Registry（主推）**：Zed 二进制之外的远程 JSON 注册表，域名 `agentclientprotocol.com`（非 zed.dev），可随时更新无需发版。
   - `crates/project/src/agent_registry_store.rs:20`：`REGISTRY_URL = "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json"`
   - `external-agents.md:35`：「This list is curated, not exhaustive.」
2. **`agent_servers` settings（完全自定义）**：用户在 `settings.json` 写任意可执行文件即可接入。
   - Schema：`crates/settings_content/src/agent.rs:668-671`、`737-769`
   - 示例（`external-agents.md:165-176`）：
     ```json
     {
       "agent_servers": {
         "my-agent": {
           "type": "custom",
           "command": "node",
           "args": ["~/projects/agent/index.js", "--acp"],
           "env": {}
         }
       }
     }
     ```
3. **Extension 提供（已废弃）**：自动迁移到 registry。源码中 `EXTENSION_TO_REGISTRY_IDS`（`agent_server_store.rs:195-209`）只是老 extension id → registry id 的迁移映射，不是可用 agent 白名单。

### Poolside 说明

Poolside 存在但**无任何专用代码**，只是普通 registry/custom agent，由其自身 CLI 写入用户配置：
```sh
pool acp setup --editor zed   # 写入 ~/.config/zed/settings.json
```

## 4. External agent 的 slash 命令如何出现

无需注册，不依赖任何设置项开关，纯运行时协议驱动：

1. 用户配置 / 从 registry 安装一个 agent → `AgentServerStore` 生成 `ExternalAgentEntry`
2. Zed 启动该 agent 子进程，建立 ACP 连接
3. Agent 主动发送 `SessionUpdate::AvailableCommandsUpdate`（`acp_thread.rs:2625-2631`）→ 存 `available_commands` + 发 `AcpThreadEvent::AvailableCommandsUpdated`
4. `SessionCapabilities::set_available_commands`（`message_editor.rs:132-134`）更新共享状态
5. 用户输入 `/` → `available_commands()`（`message_editor.rs:163-165`）→ `completion_commands()`（`message_editor.rs:111-122`）转 UI 模型

外部命令 `category == None`，在补全弹窗中归入独立分组：

```rust
// completion_provider.rs:388-394
let (key, label) = match self.category {
    Some(CommandCategory::Native) => ("commands", "Commands"),
    Some(CommandCategory::Mcp)    => ("mcp-commands", "MCP Server Commands"),
    None                          => ("acp-commands", "Commands"),  // 外部 ACP agent
};
```

排序优先级 `Native(0) < Mcp(1) < External(2)`。测试契约见 `message_editor.rs:2319-2339`。

## 5. 实例：CodeBuddy Code 外部 agent 的 slash 命令

为印证「external agent 的 `/` 命令由 agent 自身决定」这一结论，调研了本地已安装的 CodeBuddy Code（v2.133.0，路径 `~/.bun/install/global/node_modules/@tencent-ai/codebuddy-code`）。它是 Zed 中通过 ACP Registry 接入的 external agent（`~/.config/zed/settings.json` 中 `agent_servers.codebuddy-code` 为 `type: "registry"`）。

**推送机制**（源码 `sendAvailableCommandsUpdate`，位于 `dist/codebuddy-headless.js`，混淆变量名）：

```js
let el = ["/exit","/help","/config","/sandbox","/ide","/hooks","/theme","/rename",
          "/agents","/model","/model:text-to-image","/model:image-to-image","/resume",
          "/mcp","/permissions","/tasks","/plugin","/output-style","/stats","/rewind",
          "/export","/status","/bash","/memory","/add-dir","/terminal-setup","/vim",
          "/upgrade","/install-github-app","/migrate-installer"];
let ec = (await this.commandManager.getAll())
           .filter(eA => !el.includes(eA.name))
           .map(...);   // 去除本地/客户端专属命令
// 再 push 可见的 skills: {name, description, _meta:{type:"skill", source}}
```

即：**推送 `commandManager` 中所有命令，过滤掉一个黑名单（本地/客户端专属命令），再加上可见的 skills**。黑名单与文档 `acp.md` 所述「过滤掉本地命令（如 /clear、/exit）和客户端专属命令（如 /theme、/config）」一致。

**ACP 模式下实际可用的 `/` 命令**（全量命令剔除黑名单后）：

| 命令 | 作用 |
|---|---|
| `/background` (`/bg`) | 后台运行任务 |
| `/branch` | 在当前对话点创建分支 |
| `/btw` | 附加说明/旁注 |
| `/code-review` | 代码审查 |
| `/context` | 计算并显示上下文 token 分布 |
| `/copy` | 复制上一次回复到剪贴板 |
| `/cost` | 显示当前会话总成本与时长 |
| `/debug` | 启用调试日志，诊断会话问题 |
| `/deep-research` | 深度研究 |
| `/doctor` | 诊断环境与配置 |
| `/effort` | 设置回答努力程度 |
| `/feedback` | 反馈 |
| `/fork` (`/fork-bg`) | 派生 / 后台派生会话 |
| `/gateway` | 网关相关 |
| `/goal` | 设置目标 |
| `/keybindings` | 打开快捷键配置 |
| `/login` / `/logout` | 登录 / 登出 |
| `/plan` | 预览当前计划文件内容 |
| `/plugin-validate` | 校验插件目录结构与清单 |
| `/pr-comments` | 获取 GitHub PR 评论 |
| `/release-notes` | 查看发布说明 |
| `/review` | 审查 PR |
| `/simplify` | 简化 |
| `/skills` | 列出可用技能 |
| `/todos` | 显示当前会话待办列表 |
| `/verify` | 验证 |
| `/workflows` | 列出运行中和已保存的 Dynamic Workflows |

此外还推送**可见的 skills**（动态，取决于 product/skills 配置），以 `_meta:{type:"skill"}` 标记。

**印证结论**：CodeBuddy 的 slash 命令清单完全由 CodeBuddy 自身决定并通过 ACP 上报，Zed 仅是展示与路由（`category == None` 归入独立分组）。哪些命令可见、是否含本地命令，都由 CodeBuddy 的代码与配置控制，与 Zed 无关。

## 6. Zed 侧如何恢复 external agent 的历史会话

CodeBuddy 在 ACP 模式下禁用 `/resume` 是合理的：ACP 模式下「选哪个会话」由 host（Zed）通过 sidebar 完成，agent 不应再提供绕过 host 的平行会话切换入口，否则会与 Zed 的会话索引失同步。**会话恢复职责上移给 Zed，但准确说是「分工」**：

| 职责 | 归属 | 证据 |
|---|---|---|
| 记住「有哪些会话可恢复」、列表展示、点击路由 | **Zed** | `sidebar_threads` 表、`ThreadMetadataStore` |
| 实际存储对话内容 | **Agent 端** | Zed 表中无消息字段 |
| 回放历史消息 | **Agent 端** | 响应 `session/load` |
| 决定用 load 还是 resume | **Zed** | `conversation_view.rs:1112-1128` |

### ACP 协议恢复方法（无 `session/restore`）
来自 `agent-client-protocol` crate（`agent.rs:5190-5207`）：
- **`session/load`**：恢复**并回放历史消息**（能看到旧对话）
- **`session/resume`**：恢复但**不回放**，仅接续上下文
- 另有 `session/list`：把 agent 已有历史会话**导入** Zed sidebar（非每次恢复都跑）

### Zed 调度中枢
用户点 sidebar 历史条目 → `AgentPanel::load_agent_thread`（`agent_panel.rs:4371`）→ 从 `ThreadMetadataStore` 取该 thread 的 `session_id`（`agent_panel.rs:4464-4467`）→ `ConversationView`（`conversation_view.rs:1109-1143`）按 agent 能力两级降级：
```
supports_load_session()   → session/load   （带历史回放）
supports_resume_session() → session/resume （无历史 + "Resumed Session" 提示条，thread_view.rs:12144）
都不支持                  → 报错 "Loading or resuming sessions is not supported"
```
能力探测来自 agent `initialize` 上报（`acp.rs:1700-1716`）：`load_session` 看顶层 `loadSession` 布尔，`resume_session` 看 `sessionCapabilities.resume`。

### 持久化：Zed 只存索引
`sidebar_threads` 表（`thread_metadata_store.rs:1424-1437`）仅存 `session_id` + 标题 + 工作目录，**无消息体**。真实历史始终在 CodeBuddy 端。Zed 用 `thread_id`(BLOB 主键)寻址，`session_id` 为可空映射外键；未发过消息则无 `session_id`，不可恢复。

### 对 CodeBuddy 接入的实践含义
- 想让用户在 Zed 里**看到旧消息**：`initialize` 上报顶层 `loadSession: true` 并实现 `session/load`。
- 仅上报 `sessionCapabilities.resume`：降级路径，上下文接续但界面空白 + 提示条。
- 建议同时实现 `session/list`：否则用户已有的 CodeBuddy 会话不出现在 Zed sidebar，无恢复入口。

## 结论摘要

| 问题 | 结论 |
|---|---|
| 旧 slash 体系（`/file` `/now` `/docs` 等） | 已被 `76c6004b27` 整体删除，能力迁移到 `@` mention |
| 当前 Zed 硬编码 slash 命令 | 仅 `/compact` 一个，外加 2 个纯 UI 的 `/helpful`、`/not-helpful` |
| 命令注册机制 | 无静态注册表；由 agent 经 ACP `AvailableCommandsUpdate` 运行时上报 |
| External agents 是否存在 | 存在，是核心架构（ACP） |
| 谁决定有哪些 external agent | **接入方（用户）**：远程 ACP Registry + `agent_servers` settings，Zed 不硬编码白名单 |
| Poolside | 无专用代码，由其 CLI `pool acp setup --editor zed` 写用户 settings |
| Extension 提供 agent | 已废弃，自动迁移到 registry |
| External agent 的 `/` 命令 | 无需注册，agent 连上后自行上报；`category` 为 `None`，独立分组 |

> 注：若需旧 `/file`、`/terminal`、`/now` 实现细节，可从 `76c6004b27` 的父提交检出查看。
