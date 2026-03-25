# ClawBrowser 实现方案

> 轻量级无头浏览器 CLI 工具，用于网页抓取与动态内容提取  
> 技术栈：Rust + QuickJS + 自定义 DOM

---

## 1. 项目概述

ClawBrowser 是一个基于 Rust 的命令行网页抓取工具，能够：

- 抓取静态和动态网页内容
- 执行页面 JavaScript（通过 QuickJS 引擎）
- 将 HTML 转换为 Markdown 输出
- 支持单页抓取和批量并发抓取

**Release 二进制体积**：~6.5MB（单文件，无外部依赖）

---

## 2. 架构总览

```
┌─────────────────────────────────────────────┐
│                  CLI (clap)                 │
│            fetch / batch 子命令              │
├─────────────────────────────────────────────┤
│               Engine (Page)                 │
│   navigate → parse → run_js → to_markdown   │
├──────────┬──────────────┬───────────────────┤
│ HTTP     │   HTML Parser │   JS Runtime     │
│ (reqwest)│  (html5ever)  │  (rquickjs)      │
├──────────┴──────────────┴───────────────────┤
│              Custom DOM (Arena)              │
│   Node / Element / Tree / Selector / Serialize │
├─────────────────────────────────────────────┤
│           Output (htmd → Markdown)          │
└─────────────────────────────────────────────┘
```

---

## 3. 模块设计

### 3.1 `config` — 抓取配置

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `timeout_secs` | u64 | 30 | HTTP 及 JS 执行超时 |
| `wait_ms` | u64 | 5000 | 事件循环等待时间 |
| `no_js` | bool | false | 禁用 JS 执行 |
| `user_agent` | String | Chrome UA | 自定义 UA |
| `cookie` | Option | None | Cookie 头 |
| `proxy` | Option | None | HTTP/SOCKS5 代理 |
| `extra_headers` | HashMap | {} | 额外请求头 |

### 3.2 `http` — HTTP 客户端

基于 `reqwest`，支持：
- gzip / brotli / deflate 自动解压
- Cookie 存储
- 代理（HTTP/SOCKS5）
- 自动重定向（最多 10 次）
- 可配置超时

### 3.3 `dom` — 自定义 DOM 实现

采用 **Arena 分配器** 模式，所有节点存储在连续 `Vec<NodeData>` 中，通过 `NodeId(u32)` 索引访问。

```
Arena (Vec<NodeData>)
  ├── NodeData { id, node_type, parent, first_child, last_child, prev_sibling, next_sibling }
  │     └── NodeType::Element(ElementData { tag, attributes, id_cache })
  │     └── NodeType::Text(String)
  │     └── NodeType::Comment(String)
  │     └── NodeType::Document
  │     └── NodeType::DocType { name, public_id, system_id }
  └── ...
```

**子模块**：

| 模块 | 职责 |
|------|------|
| `arena.rs` | 节点分配（alloc）、索引访问（get/get_mut） |
| `node.rs` | NodeId、NodeType、ElementData、Attribute 定义 |
| `document.rs` | Document 结构体，便捷方法（body/head/title/getElementById 等） |
| `tree.rs` | 树操作（appendChild、insertBefore、detach、children、cloneNode 等） |
| `selector.rs` | CSS 选择器引擎（tag、#id、.class、[attr]、后代组合器） |
| `serialize.rs` | DOM → HTML 序列化（innerHTML、outerHTML、完整文档） |

### 3.4 `html` — HTML 解析器

基于 `html5ever`，实现 `TreeSink` trait 将解析结果直接写入自定义 Arena DOM。

关键设计：
- 使用 `RefCell<Inner>` 实现内部可变性
- `elem_names` HashMap 配合 `unsafe` 裸指针解引用解决 `ExpandedName<'a>` 生命周期问题
- 文本节点自动合并（相邻文本追加而非创建新节点）

### 3.5 `js` — QuickJS 集成

#### 3.5.1 Runtime (`runtime.rs`)

- QuickJS 异步运行时（`AsyncRuntime` + `AsyncContext`）
- 栈大小限制：512KB
- 内存限制：64MB
- 支持 interrupt handler 中断无限循环

#### 3.5.2 Bridge 架构（核心设计决策）

为解决 `rquickjs::Value<'js>` 生命周期不变性问题，采用 **Rust Bridge + JS Shim** 双层架构：

```
JavaScript 代码
     ↓ 调用
DOM Shim (dom_shim.js)   ← JS 层：构建完整 DOM API 对象
     ↓ 调用
__dom Bridge (dom.rs)    ← Rust 层：只返回原始类型 (u32/String/bool/Vec<u32>)
     ↓ 操作
Arena DOM (tree.rs)      ← 实际 DOM 数据
```

**为什么这样设计**：
- Rust 闭包中直接返回 `rquickjs::Value` 会遇到生命周期不变性错误
- Bridge 只传递原始类型（NodeId 为 u32，-1 表示 null），完全避免生命周期问题
- JS Shim 负责用 `Object.defineProperty` 构建完整的 DOM 对象包装

#### 3.5.3 绑定模块

| 模块 | 暴露的全局对象/函数 |
|------|---------------------|
| `console.rs` | `console.log/info/warn/error/debug` → tracing 日志 |
| `timer.rs` | `setTimeout/setInterval/clearTimeout/clearInterval/requestAnimationFrame` |
| `fetch.rs` | `fetch()` API（通过 `block_in_place` + 同步桥） |
| `dom.rs` | `__dom` 桥对象（30+ 方法） |
| `window.rs` | `window/location/navigator/screen/performance/atob/btoa` |
| `dom_shim.js` | `document/Event/CustomEvent/MutationObserver/localStorage` 等完整 JS API |

#### 3.5.4 Timer 实现

- `BinaryHeap<Reverse<TimerEntry>>` 最小堆，按触发时间排序
- `Persistent<Function<'static>>` 存储回调，跨 async 边界安全传递
- `unsafe impl Send for TimerEntry` — 因为 Persistent 仅在同一 JS 运行时上下文中访问

#### 3.5.5 Fetch 实现

```
JS: fetch(url) → Promise
      ↓
JS Shim: 调用 __fetch_bridge.doFetch(url, method, body)
      ↓
Rust: tokio::task::block_in_place → Handle::block_on → reqwest 异步请求
      ↓
返回: Vec<String> ["ok", status, body] 或 ["error", message]
      ↓
JS Shim: 包装为 Response 对象，resolve Promise
```

使用 `block_in_place` 而非直接 `block_on` 是为了避免在 tokio 任务内嵌套阻塞导致死锁。

#### 3.5.6 DOM Shim (`dom_shim.js`)

完整实现的 JS 侧 DOM API，约 700 行：

- **事件系统**：addEventListener/removeEventListener/dispatchEvent，支持 capture/once
- **Event/CustomEvent** 构造函数
- **MutationObserver** 基础 stub
- **Node/Element 包装**：通过 `wrapNode(nid)` 为每个 NodeId 创建 JS 对象
  - 属性：innerHTML/outerHTML/textContent/className/id/children/parentNode 等
  - 方法：querySelector/querySelectorAll/appendChild/removeChild/cloneNode/closest/matches 等
  - classList、style（cssText）、dataset 代理
  - getBoundingClientRect（返回零值）
- **Document 对象**：getElementById/createElement/createTextNode/createDocumentFragment 等
- **Window stub**：getComputedStyle/matchMedia/localStorage/sessionStorage/history/XMLHttpRequest/Image 等
- **事件触发**：`__fireDOMContentLoaded()` / `__fireLoad()` 由 Rust 在脚本执行后调用

### 3.6 `engine` — 页面引擎

`Page::navigate(url, config)` 核心流程：

```
1. HTTP 请求获取 HTML
2. html5ever 解析为 DOM 树
3. 如果 no_js=true → 直接返回
4. 如果 no_js=false：
   a. 创建 QuickJS runtime
   b. 设置 interrupt handler（AtomicBool 中断标志）
   c. 安装所有 JS 绑定（console/window/dom/timer/fetch/shim）
   d. 遍历 <script> 标签，逐个加载执行
      - 外部脚本：通过 reqwest 下载
      - 内联脚本：直接 eval
      - 执行失败记录日志但继续
      - 总超时保护（timeout_secs）
   e. 触发 DOMContentLoaded 事件
   f. 触发 load 事件
   g. 运行事件循环（处理 timer 回调和 pending jobs）
      - 循环超时 = wait_ms
      - 空闲 10 轮退出
   h. 设置中断标志，释放 JS runtime
   i. Arc::try_unwrap 获取 Document 所有权
      - 失败时回退：序列化 HTML → 重新解析
5. 输出 Markdown 或 HTML
```

**错误恢复**：JS 执行完全失败时，回退到静态 DOM（重新解析原始 HTML），保证始终返回有效结果。

### 3.7 `output` — Markdown 转换

基于 `htmd` 库，自动跳过 `<script>/<style>/<noscript>/<svg>` 标签。

---

## 4. 并发模型

批量抓取使用 `tokio::task::LocalSet` + `spawn_local`：

```
LocalSet (单线程)
  ├── spawn_local(fetch url_0)  ←┐
  ├── spawn_local(fetch url_1)  ←┤ Semaphore 控制并发数
  ├── spawn_local(fetch url_2)  ←┘
  └── ...
```

**为什么用 LocalSet 而非 tokio::spawn**：
- QuickJS 的 `AsyncRuntime` / `Persistent` 包含裸指针，不满足 `Send` trait
- `LocalSet` 允许在单线程上并发执行非 `Send` future
- `Semaphore` 控制实际并发数量

---

## 5. 关键技术决策

| 决策 | 选择 | 原因 |
|------|------|------|
| JS 引擎 | QuickJS (rquickjs) | 轻量、可嵌入、编译快、无外部依赖 |
| DOM 存储 | Arena (Vec) | 缓存友好、无 GC、NodeId 为 u32 轻量传递 |
| JS↔Rust 交互 | Bridge + Shim | 避免 Value 生命周期问题，解耦清晰 |
| 异步运行时 | tokio (full) | 生态完善，reqwest 原生支持 |
| HTML 解析 | html5ever | 标准 HTML5 解析算法，容错性强 |
| HTTP 客户端 | reqwest 0.12 | rustls-tls 跨平台、支持代理/压缩/Cookie |
| Markdown 转换 | htmd | 轻量、可配置跳过标签 |
| CLI 框架 | clap 4 (derive) | 声明式、自动生成帮助 |

---

## 6. Release 构建优化

```toml
[profile.release]
opt-level = 3        # 最大优化
strip = true          # 去除调试符号
lto = true            # 链接时优化
codegen-units = 1     # 单代码生成单元（更好优化）
panic = "abort"       # panic 直接 abort（减小体积）
```

---

## 7. 测试覆盖

17 个集成测试，覆盖：

| 类别 | 测试项 |
|------|--------|
| DOM 解析 | 基本 HTML、嵌套元素、畸形 HTML |
| 元素查找 | getElementById、querySelector、querySelectorAll |
| 序列化 | HTML 往返、innerHTML |
| DOM 操作 | 创建/追加元素、属性读写、节点移除、浅/深克隆 |
| Markdown | 基本转换、链接保留、脚本过滤 |
| 网络集成 | fetch example.com (no-js) |

---

## 8. CLI 使用

```bash
# 单页抓取（Markdown 输出）
clawbrowser fetch https://example.com

# 带 JS 执行
clawbrowser fetch https://example.com --wait 3000

# 禁用 JS，输出 HTML
clawbrowser fetch https://example.com --no-js --html

# 输出到文件
clawbrowser fetch https://example.com -o output.md

# 自定义 UA + Cookie + 代理
clawbrowser fetch https://example.com --user-agent "MyBot/1.0" --cookie "sid=abc" --proxy socks5://127.0.0.1:1080

# 批量抓取
clawbrowser batch urls.txt -o ./output --concurrency 5

# 详细日志
clawbrowser fetch https://example.com -v
```

---

## 9. 项目文件结构

```
clawbrowser/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI 入口，fetch/batch 命令
│   ├── lib.rs               # 模块导出
│   ├── config.rs            # FetchConfig 配置结构
│   ├── http/
│   │   └── mod.rs           # HttpClient (reqwest 封装)
│   ├── html/
│   │   ├── mod.rs
│   │   └── parser.rs        # html5ever TreeSink → Arena DOM
│   ├── dom/
│   │   ├── mod.rs           # 公共导出
│   │   ├── arena.rs         # Arena 分配器
│   │   ├── node.rs          # NodeId/NodeType/ElementData/Attribute
│   │   ├── document.rs      # Document 便捷方法
│   │   ├── tree.rs          # 树操作 (appendChild/detach/cloneNode...)
│   │   ├── selector.rs      # CSS 选择器引擎
│   │   └── serialize.rs     # DOM → HTML 序列化
│   ├── js/
│   │   ├── mod.rs
│   │   ├── runtime.rs       # QuickJS AsyncRuntime 封装
│   │   ├── event_loop.rs    # 事件循环
│   │   ├── script_loader.rs # <script> 提取与加载
│   │   └── bindings/
│   │       ├── mod.rs       # install_all + dom_shim 加载
│   │       ├── console.rs   # console.log → tracing
│   │       ├── timer.rs     # setTimeout/setInterval (BinaryHeap)
│   │       ├── fetch.rs     # fetch() API (block_in_place)
│   │       ├── dom.rs       # __dom Bridge (30+ Rust 函数)
│   │       ├── window.rs    # window/location/navigator/screen/atob/btoa
│   │       └── dom_shim.js  # JS 侧完整 DOM API (~700行)
│   └── output/
│       ├── mod.rs
│       └── markdown.rs      # HTML → Markdown (htmd)
└── tests/
    └── integration.rs       # 17 个集成测试
```

---

## 10. 依赖一览

| crate | 版本 | 用途 |
|-------|------|------|
| clap | 4 | CLI 参数解析 |
| tokio | 1 (full) | 异步运行时 |
| reqwest | 0.12 | HTTP 客户端 |
| html5ever | 0.29 | HTML5 解析 |
| markup5ever | 0.14 | html5ever 辅助类型 |
| selectors | 0.26 | CSS 选择器（备用） |
| rquickjs | 0.11 | QuickJS Rust 绑定 |
| htmd | 0.5 | HTML → Markdown |
| tracing | 0.1 | 结构化日志 |
| tracing-subscriber | 0.3 | 日志输出格式化 |
| serde / serde_json | 1 | 序列化 |
| anyhow | 1 | 错误处理 |
| url | 2 | URL 解析 |
| cssparser | 0.34 | CSS 解析（备用） |
