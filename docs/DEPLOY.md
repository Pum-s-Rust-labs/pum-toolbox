# Hướng dẫn Deploy Miễn Phí (100% Free Tier)

`pum-toolbox` được đóng gói thành **1 container Docker duy nhất** (Axum phục vụ cả API lẫn giao diện React đã build), kết nối trực tiếp tới PostgreSQL bên ngoài của bạn nên có thể deploy miễn phí cực gọn trên nhiều nền tảng.

## So sánh 3 lựa chọn Free tốt nhất

| Nền tảng | Chi phí / Thẻ | Điểm mạnh | Hạn chế cần biết |
| --- | --- | --- | --- |
| **1. Render (Khuyên dùng)** | Miễn phí, **không cần thẻ tín dụng** | Hỗ trợ sẵn `render.yaml` Blueprint, có vùng **Singapore**, HTTPS tự động, deploy tự động khi push GitHub | Service Free ngủ sau 15 phút không có truy cập (mở lại mất ~20–30s cold start; Rust khởi động rất nhanh) |
| **2. Hugging Face Spaces (Docker)** | Miễn phí, **không cần thẻ** | Tài nguyên khủng (**2 vCPU, 16 GB RAM**), HTTPS miễn phí, ngủ sau 48 giờ không dùng | URL dạng `*.hf.space`, cần mở cổng `7860` (`PORT=7860`) |
| **3. Koyeb (Free Web Service)** | Miễn phí (cần xác minh thẻ hoặc GitHub Student) | Vùng gần châu Á, chạy mượt Docker Rust | Có thể yêu cầu thẻ để chống spam |

---

## Cách 1: Deploy lên Render trong 2 phút (Khuyên dùng)

1. Đăng nhập [Render](https://render.com) bằng tài khoản GitHub (`Pum283`).
2. Chọn **New + → Blueprint** (hoặc **Web Service → Build and deploy from a Git repository**).
3. Chọn repo **`Pum-s-Rust-labs/pum-toolbox`** (Render sẽ tự đọc file `render.yaml` và `Dockerfile`).
4. Điền các biến môi trường bí mật (Environment Variables):
   - `DATABASE_URL`: `postgres://<USER>:<URL_ENCODED_PASSWORD>@<HOST>:5432/<DATABASE>?sslmode=prefer`
   - `PUBLIC_BASE_URL`: `https://<ten-app-cua-ban>.onrender.com`
   - `CORS_ALLOWED_ORIGINS`: `https://<ten-app-cua-ban>.onrender.com`
   - `BOOTSTRAP_ADMIN_PASSWORD`: đặt mật khẩu mạnh cho tài khoản `superadmin`
   - *(Render tự sinh sẵn `JWT_SECRET` dài an toàn và chạy ở vùng `singapore`)*
5. Nhấn **Deploy** → xong là truy cập ngay bằng HTTPS!

---

## Cách 2: Deploy lên Hugging Face Spaces (Docker, 16 GB RAM Free)

1. Tạo Space mới trên [Hugging Face Spaces](https://huggingface.co/new-space) chọn SDK **Docker (Blank)**.
2. Đẩy mã nguồn hoặc đồng bộ GitHub Actions lên Space, đặt biến môi trường `PORT=7860`, `DATABASE_URL`, `JWT_SECRET`, `PUBLIC_BASE_URL`, `CORS_ALLOWED_ORIGINS`, `BOOTSTRAP_ADMIN_PASSWORD` trong mục **Settings → Variables and secrets**.
3. Khởi chạy xong ứng dụng sẵn sàng qua HTTPS và có tận 16 GB RAM cho ImageLab xử lý ảnh hàng loạt.
