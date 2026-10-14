# KeyBox

本地账号密码管理工具，Tauri 2 + Rust + React，支持 macOS / Windows。

## 功能
- **免密码打开**：面向私人电脑，无主密码、无锁屏，数据以明文保存在本机 SQLite。
- **智能添加**（⌘N / Ctrl+N）：粘贴任意文本自动识别地址、端口、账号、密码，支持中文标签与中文/特殊字符密码，空行分隔可批量添加。
  - `gz-cdb-example.sql.tencentcdb.com port:25729 user root password Demo.456!@#$%^&*()`
  - `网址：https://mail.qq.com 账号：张三 密码：我的密码`
  - `mysql://admin:p@ss@db.example.com:3307/app`
  - `10.1.2.3:22 root 123456`
- **JSON 导入/导出**：默认明文导出，可选设置导出密码加密（Argon2id + XChaCha20-Poly1305）；导入时可选择跳过/覆盖/保留重复项。
- 可选复制密码后自动清空剪贴板、强密码生成、收藏、标签、搜索。
- Mac 风格界面：毛玻璃侧边栏、红绿灯、跟随系统深色模式。

## 快捷键
| 操作 | macOS | Windows |
|---|---|---|
| 智能添加 | ⌘N | Ctrl+N |
| 手动新建 | ⌘⇧N | Ctrl+Shift+N |
| 搜索 | ⌘F | Ctrl+F |
| 设置 | ⌘, | Ctrl+, |

## 开发
```bash
npm install
npm run tauri dev          # 启动应用
cd src-tauri && cargo test --lib   # 后端单元测试（解析器、加密、导入导出）
```
浏览器预览 UI（不连后端）：`npm run dev` 后打开 `http://localhost:1420/?mock`。

### Windows 本机开发与测试
安装 Node.js 22.12+、Rust（MSVC 工具链）、Visual Studio 2022 Build Tools 的“使用 C++ 的桌面开发”和 Windows SDK，并安装 WebView2 Runtime。安装后重新打开终端，确认 `cargo --version` 可用。

在项目根目录使用 PowerShell：
```powershell
npm ci
npm run build
cargo test --lib --locked --manifest-path src-tauri/Cargo.toml
npm run tauri dev
# 构建 Windows 安装包
npm run tauri build -- --bundles nsis,msi
```
仅生成可直接运行的正式版：`npm run tauri build -- --no-bundle`，然后双击 `src-tauri\target\release\keybox.exe`。此方式不会安装应用或创建开始菜单快捷方式。
若 Rust 已安装但当前终端找不到 `cargo`，可临时执行 `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`，然后重试。
JSON 导入支持 UTF-8（含 BOM）、Windows CRLF 换行及中文文件路径。

## 打包
- macOS：`npm run tauri build`（产物在 `src-tauri/target/release/bundle/`）
- Windows（在 Mac 上交叉编译，仅产出 NSIS `.exe` 安装包）：
  ```bash
  brew install nsis llvm && rustup target add x86_64-pc-windows-msvc && cargo install --locked cargo-xwin
  PATH="/opt/homebrew/opt/llvm/bin:$PATH" npm run tauri build -- --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
  ```
  产物：`src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`。也可推送 `v*` 标签由 GitHub Actions 构建 `.msi` / `.exe`。

## 数据位置
- macOS：`~/Library/Application Support/com.keybox.app/keybox.db`
- Windows：`%APPDATA%\com.keybox.app\keybox.db`

⚠️ 数据库为明文，任何能访问这台电脑的人都能读取，请仅在私人电脑上使用并定期导出备份。
