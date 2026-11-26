# Kiến trúc Workspace & Hướng mở rộng đa nền tảng

Repo này được thiết kế theo mô hình **Cargo Workspace** với nguyên tắc cốt lõi:

> **Tách logic nghiệp vụ (`crates/core`) ra khỏi lớp giao diện / nền tảng (`crates/app`, `crates/api`, `crates/desktop`, `crates/mobile`, ...).**

Nhờ đó, dù sau này bạn xây dựng web app, công cụ CLI, ứng dụng Windows hay ứng dụng di động, toàn bộ xử lý dữ liệu, model, validate và business rules trong `crates/core` đều có thể **tái sử dụng 100%**.

## Sơ đồ cấu trúc mở rộng

```text
my-project/
├── Cargo.toml                 # Khai báo workspace, dependency chung, lint chung
└── crates/
    ├── core/                  # Logic thuần Rust (không phụ thuộc UI/Web/OS)
    ├── app/                   # Binary mặc định (CLI hoặc runner)
    ├── api/                   # (Tuỳ chọn) Web backend / REST API (Axum / Actix)
    ├── desktop/               # (Tuỳ chọn) Windows / Desktop app (Tauri / egui / iced)
    └── mobile/                # (Tuỳ chọn) Mobile binding (Tauri Mobile / UniFFI)
```

## Gợi ý stack Rust theo từng loại dự án

| Loại dự án                     | Thư viện / Framework phổ biến trong Rust             | Cách tích hợp vào workspace                               |
| ------------------------------ | ---------------------------------------------------- | --------------------------------------------------------- |
| **CLI / Automation Tool**      | `clap`, `indicatif`, `ratatui`                       | Dùng trực tiếp `crates/app` và gọi `app-core`             |
| **Web Backend / API**          | `axum`, `tokio`, `tower-http`, `sqlx`                | Thêm `crates/api` phụ thuộc vào `app-core`                |
| **Web Frontend (WASM)**        | `leptos`, `dioxus`, `yew`                            | Thêm `crates/web` biên dịch sang `wasm32-unknown-unknown` |
| **Windows / Desktop App**      | `tauri` (HTML/JS/TS + Rust), `egui`, `iced`, `slint` | Thêm `crates/desktop` (hoặc `src-tauri`) gọi `app-core`   |
| **Mobile App (iOS / Android)** | `tauri` v2 Mobile, `uniffi`, `flutter_rust_bridge`   | Thêm `crates/mobile` xuất FFI từ `app-core`               |

## Quy ước phát triển chung trong Pum's Rust Labs

1. **Không dùng `unsafe` trừ khi thực sự cần thiết** (mặc định workspace bật `unsafe_code = "forbid"`).
2. **Tầng thư viện (`core`) dùng `thiserror`** để định nghĩa lỗi rõ ràng; **tầng ứng dụng (`app`) dùng `anyhow`** để gom và hiển thị ngữ cảnh lỗi dễ đọc.
3. **Khai báo dependency dùng chung tại `[workspace.dependencies]`** trong `Cargo.toml` gốc để đồng bộ phiên bản giữa các crate.
4. **Luôn chạy `fmt`, `clippy`, `test`** trước khi tạo Pull Request.
