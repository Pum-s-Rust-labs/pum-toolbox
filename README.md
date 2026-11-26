# rust-base-template

Source base chuẩn bằng ngôn ngữ **Rust** cho các dự án thuộc tổ chức **[Pum's Rust Labs](https://github.com/Pum-s-Rust-labs)**.

Template này dùng kiến trúc **Cargo Workspace** gọn nhẹ, dễ tiếp cận cho người mới bắt đầu với Rust nhưng vẫn đủ chuẩn mực để mở rộng sang **CLI tool, Web backend, ứng dụng Windows/Desktop, WebAssembly và Mobile**.

## Cấu trúc thư mục

```text
rust-base-template/
├── .github/
│   ├── PULL_REQUEST_TEMPLATE.md
│   └── workflows/ci.yml        # Tự động kiểm tra fmt, clippy, build & test (Linux + Windows)
├── crates/
│   ├── core/                   # Crate thư viện `app-core`: logic nghiệp vụ, config, error handling
│   └── app/                    # Crate binary `app`: điểm khởi chạy mẫu dùng tracing + anyhow
├── docs/
│   └── ARCHITECTURE.md         # Hướng dẫn mở rộng sang Web, CLI, Windows app, Mobile app
├── Cargo.toml                  # Cấu hình Cargo Workspace, dependencies & lints dùng chung
├── rust-toolchain.toml         # Khóa kênh stable kèm rustfmt và clippy
├── rustfmt.toml                # Quy chuẩn format code
├── clippy.toml                 # Quy chuẩn lint
└── .env.example                # Biến môi trường mẫu
```

## Tính năng có sẵn

- **Cargo Workspace (Rust 2024 edition)**: quản lý tập trung phiên bản, dependencies và lints (`clippy::pedantic`, `unsafe_code = "forbid"`).
- **Xử lý lỗi chuẩn Rust**: `thiserror` ở tầng thư viện (`app-core`) và `anyhow` ở tầng ứng dụng (`app`).
- **Logging có cấu trúc**: `tracing` + `tracing-subscriber` hỗ trợ lọc log bằng biến môi trường `RUST_LOG`.
- **Cấu hình an toàn & dễ test**: đọc cấu hình qua `AppConfig::from_env()` kèm unit test chuẩn.
- **GitHub Actions CI**: tự động kiểm tra `cargo fmt`, `cargo clippy`, `cargo test` và `cargo build` trên cả **Ubuntu** và **Windows**.

## Cách sử dụng làm dự án mới

1. Nhấn **Use this template** → **Create a new repository** trên GitHub trong tổ chức `Pum-s-Rust-labs`.
2. Clone repository mới về máy:
   ```bash
   git clone https://github.com/Pum-s-Rust-labs/<ten-du-an-moi>.git
   cd <ten-du-an-moi>
   ```
3. Cập nhật thông tin dự án trong `Cargo.toml` (`repository`, tên crate nếu cần) và viết lại `README.md`.
4. Xem thêm [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) khi muốn thêm crate mới cho Web, Desktop hoặc Mobile.

## Các lệnh Cargo thường dùng

```bash
# Chạy ứng dụng mẫu
cargo run -p app

# Chạy ứng dụng với log debug
RUST_LOG=debug cargo run -p app

# Chạy toàn bộ unit test trong workspace
cargo test --workspace

# Format code theo chuẩn rustfmt.toml
cargo fmt --all

# Kiểm tra lỗi tiềm ẩn và gợi ý cải thiện bằng Clippy
cargo clippy --workspace --all-targets -- -D warnings

# Build bản release tối ưu dung lượng & tốc độ
cargo build --release --workspace
```

## Giấy phép

Phát hành theo giấy phép [MIT](LICENSE).
