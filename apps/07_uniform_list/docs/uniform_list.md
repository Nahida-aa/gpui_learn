# uniform_list —— 定高虚拟列表

本文记录 `07_uniform_list` 用 GPUI `uniform_list` 做长列表 / 滚动渲染的练习，
以及把官方最简示例补齐成「真实 app 可用」形态时踩的坑。**实战经验，设备 /
桌面均验证。**

---

## 1. uniform_list 是什么

`uniform_list` 是 GPUI 里**所有 item 等高**的虚拟列表构件：

- 只渲染**可视区**内的 item，列表再长也不会把几千个 div 都建出来；
- 滚动由 GPUI 内部处理，外部只管「给第 ix 个 item 长什么样」；
- 适合：消息列表、文件列表、命令面板、日志等定高行。

不等高的列表要用 `list`（非 uniform），本文不涉。

---

## 2. 关键 API（当前 gpui rev 82aef44）

```rust
uniform_list(
    "entries",                          // 列表 id（同 window 内唯一）
    self.items.len(),                  // item 总数（定高 ⇒ 总高度 = count * 单行高）
    cx.processor(|this, range, _window, cx| {
        // range：当前需要渲染的行号区间（虚拟化的核心）
        // 只构造 [range.start, range.end) 内的元素，区间外不渲染
        range.map(|ix| div().id(ix).h(px(50.0)).child(format!("Item {ix}"))).collect()
    }),
)
.track_scroll(&self.scroll_handle)     // 可选：把滚动状态接到 handle，键盘导航时跟随
.h_full();
```

- **虚拟化**：processor 的 `range` 随滚动变化，GPUI 只问可视区要元素。不要在
  processor 里做「渲染全部」的假设——`range` 之外你是拿不到也不该构造的。
- **`UniformListScrollHandle`**：`scroll_to_item(ix, ScrollStrategy::Nearest)`
  把第 ix 项滚到可视区。`Nearest` = 不到边界不滚、到边界才滚（体验最自然）。
  键盘上下导航要让选中项始终可见，就靠它。
- **`track_focus` + 键盘 action**：列表容器要 `track_focus(&focus_handle)` 才能
  收到键盘事件（↑/↓/Enter）。聚焦后才能 `on_action(cx.listener(...))`。

---

## 3. 本例在官方示例基础上补全了什么

官方自带示例（`crates/gpui/examples/uniform_list.rs`）只演示「渲染 + 点击
`println!`」。真实 app 还必需：

1. **可变数据源**：item 列表存进 view 字段（`Vec<String>`），可动态增删，而非
   硬编码 `ix + 1`。
2. **选中态高亮**：`selected: usize` 记当前选中项，processor 里 `ix == selected`
   时换底色（蓝底白字）。
3. **键盘导航**：自定义 `SelectNext` / `SelectPrev` action（绑定 ↑/↓），循环到
   头尾；`Confirm` action 绑定 Enter。`select_next/prev` 里调 `select()` 统一
   走「更新 selected + scroll_to_item + notify」。
4. **点击选中**：item `.on_click(...)` 也走同一个 `select()`。

`select()` 统一三件事：更新 `selected`、滚到可视区、刷新（`window.refresh()`
+ `cx.notify()`）。

---

## 4. 踩的坑

### 坑 1：`processor` 闭包必须 `'static` —— 点击不能直接捕获 `this`

**现象**：`cx.processor(|this, range, ...| { ... .on_click(cx.listener(|this, ...| this.select(ix)) ) })` 编译报
`borrowed data escapes ... argument requires 'static`。

**根因**：`Context::processor` 的闭包签名要求 `+ 'static`，且 GPUI 会把这个
闭包持久化（跨帧渲染用）。processor 闭包内部的 `on_click` 若用
`cx.listener`（这里的 `cx` 是 processor 的参数 `&mut Context`，带 render 的
生命周期 `'2`），会把 `'2` 带进 'static 闭包，违反约束。

**修复**：点击要更新 view 时，**不要捕获 `this`（它是 `&mut Self` 借用）**。
改用 `cx.entity()` 拿到 `Entity<Self>`（'static，可 move 进 'static 闭包），
在 `on_click` 里 `entity.update(cx, |this, cx| this.select(ix, ...))`：

```rust
cx.processor(|this, range, _window, cx| {
    let entity = cx.entity();          // 'static，可 move
    range.map(|ix| {
        div().id(ix).on_click({
            let entity = entity.clone();
            move |_event, window, cx| {
                entity.update(cx, |this, cx| this.select(ix, window, cx));
            }
        })
        // ...
    }).collect()
})
```

同理，processor 闭包里读 `this.selected` / `this.items` 做高亮是**可以的**
（只读字段、不逃逸），只有「要把 `this` 的 `&mut` 借进 'static 闭包」才不行。

### 坑 2：键盘事件收不到 —— 忘了 `track_focus`

列表容器不 `track_focus` 就不会成为 keydown 的目标，`on_action` 永远不触发。
先 `track_focus(&self.focus_handle)`，必要时 `window.focus(&focus_handle, cx)`
主动聚焦。

### 坑 3：`range` 类型要显式标注

`cx.processor(|this, range, ...|)` 的 `range` 编译器推不出类型，需写成
`range: Range<usize>`，并 `use std::ops::Range;`。

### 坑 4：`actions!` 上方不能写 doc 注释

`///` 写在 `actions!(...)` 宏调用上方会被当成「宏不产生文档」的无效注释并告警。
把说明改成普通 `//` 注释写在宏下方或上方（不带 `///`）即可。

---

## 5. 运行

```bash
cargo run -p uniform_list_07
```

窗口 360×520，顶部一行状态（项数 / 选中项 / 操作提示），下方 100 行虚拟列表：
- 鼠标点击任意行 → 选中高亮；
- ↑/↓ → 循环导航并自动滚到可视区；
- Enter → 确认（logcat/终端打 `[uniform_list] confirmed item N=...`）。
