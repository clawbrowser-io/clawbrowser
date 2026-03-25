# ClawBrowser 运行与使用指南

## 1. 编译

```bash
# 开发模式（编译快，体积大）
cd d:\Project\ai-browser\clawbrowser
cargo build

# Release 模式（编译慢，体积小 ~6.5MB，性能好）
cargo build --release
```

编译产物位置：
- 开发：`target\debug\clawbrowser.exe`
- Release：`target\release\clawbrowser.exe`

---

## 2. 单页抓取（`fetch` 命令）

**基本用法 — 输出 Markdown 到终端：**
```bash
clawbrowser fetch https://example.com
```

**禁用 JS 执行（纯静态抓取，速度快）：**
```bash
clawbrowser fetch https://example.com --no-js
```

**启用 JS 并自定义等待时间（等待动态内容加载）：**
```bash
clawbrowser fetch https://example.com --wait 3000
```

**输出到文件：**
```bash
clawbrowser fetch https://example.com -o result.md
```

**输出完整 HTML 而非 Markdown：**
```bash
clawbrowser fetch https://example.com --html
```

**输出 HTML 到文件：**
```bash
clawbrowser fetch https://example.com --html -o page.html
```

**详细日志模式（调试用）：**
```bash
clawbrowser fetch https://example.com -v
```

**设置超时：**
```bash
clawbrowser fetch https://example.com --timeout 15
```

**自定义 User-Agent：**
```bash
clawbrowser fetch https://example.com --user-agent "MyBot/1.0"
```

**带 Cookie：**
```bash
clawbrowser fetch https://example.com --cookie "session=abc123; token=xyz"
```

**额外请求头：**
```bash
clawbrowser fetch https://example.com --header "Authorization:Bearer xxx" --header "X-Custom:value"
```

**通过代理访问：**
```bash
clawbrowser fetch https://example.com --proxy socks5://127.0.0.1:1080
clawbrowser fetch https://example.com --proxy http://proxy.example.com:8080
```

**组合使用：**
```bash
clawbrowser fetch https://example.com -v --wait 5000 --timeout 20 --proxy socks5://127.0.0.1:7890 -o output.md
```

---

## 3. 批量抓取（`batch` 命令）

先准备一个 URL 文件，每行一个 URL，`#` 开头为注释：

```text
# urls.txt
https://example.com
https://httpbin.org/html
https://www.rust-lang.org
```

**批量抓取：**
```bash
clawbrowser batch urls.txt -o ./output
```

**控制并发数：**
```bash
clawbrowser batch urls.txt -o ./output --concurrency 5
```

**批量抓取禁用 JS：**
```bash
clawbrowser batch urls.txt -o ./output --no-js --concurrency 10
```

输出文件命名为 `0000.md`、`0001.md`、`0002.md` ...，保存在指定目录下。

---

## 4. 运行测试

```bash
# 运行全部 17 个集成测试
cargo test

# 只运行特定测试
cargo test parse_basic_html
cargo test fetch_example_com
```

---

## 5. 完整参数参考

```bash
# 查看帮助
clawbrowser --help
clawbrowser fetch --help
clawbrowser batch --help
```

| 参数 | fetch | batch | 说明 |
|------|:-----:|:-----:|------|
| `--timeout, -t` | ✓ | ✓ | JS 执行超时（秒），默认 30 |
| `--wait, -w` | ✓ | ✓ | 页面加载等待（毫秒），默认 5000 |
| `--no-js` | ✓ | ✓ | 禁用 JavaScript |
| `--user-agent` | ✓ | ✓ | 自定义 User-Agent |
| `--proxy` | ✓ | ✓ | 代理地址 |
| `--verbose, -v` | ✓ | ✓ | 详细日志 |
| `--output, -o` | ✓ | ✓ | 输出文件/目录 |
| `--cookie` | ✓ | ✗ | Cookie 值 |
| `--header` | ✓ | ✗ | 额外请求头 (K:V) |
| `--html` | ✓ | ✗ | 输出 HTML 而非 Markdown |
| `--concurrency` | ✗ | ✓ | 并发数，默认 10 |
