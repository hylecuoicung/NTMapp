# Quy tắc dựng của team

Áp dụng cho mọi lần dựng qua MCP, cho mọi team. Người dùng dặn khác trong cuộc trò chuyện thì làm theo người dùng. Quy tắc này đứng trên thói quen dựng của riêng bạn.

## 1. Trước khi cắt

- Không sửa sequence gốc của người dùng. Làm trên bản sao của nó (Duplicate) hoặc một sequence mới tên "<tên gốc> - Rough cut".
- Xác định loại nội dung, vì các bước bên dưới khác nhau:
  - **Cảnh diễn:** phim, quảng cáo, có kịch bản, có đạo diễn hô.
  - **Nói trước camera:** phỏng vấn, vlog, livestream, hướng dẫn.
  - Không chắc thì hỏi người dùng.
- Tạo transcript bằng `transcript.generate` với `"language": "vi"` và `"diarize": true` (để có nhãn người nói).
  - App mặc định là tiếng Anh: thiếu `"vi"` thì tiếng Việt ra sai hết. Thoại toàn tiếng Anh thì dùng `"en"`.
  - Model `whisper-small` nghe tiếng Việt tốt hơn `whisper-base`. Chưa có thì hỏi người dùng trước khi tải (`transcript.models` cho biết dung lượng), rồi tải bằng `transcript.downloadModel`.

## 2. Tiếng hô của đạo diễn

Đây là lỗi hay gặp nhất: tiếng hô của đạo diễn còn dính trong bản dựng.

Các từ hay gặp. Whisper có thể viết theo nhiều kiểu, nên tìm cả các biến thể:

- **Bắt đầu:** "action", "ác-sần", "ắc sần", "diễn", "bắt đầu", "máy", "máy chạy", "rolling", "camera", "speed", "chuẩn bị", "vào vị trí", "im lặng", "yên lặng", "một hai ba", "3 2 1".
- **Kết thúc:** "cut", "cắt", "cát", "kát", "dừng", "stop".
- **Nhận xét sau take:** "ok", "okay", "oke", "ô kê", "được", "được rồi", "tốt", "đẹp", "qua", "lại", "làm lại", "lần nữa", "thêm lần nữa".
- **Đọc số cảnh/take:** "take …", "cảnh … lần …".

Cách cắt:

- Phần dùng được của một take nằm giữa tiếng "action" (hoặc "diễn") và tiếng "cut" (hoặc "cắt") của đạo diễn.
  - Bỏ từ đầu clip đến trước câu thoại đầu tiên sau "action".
  - Bỏ từ tiếng "cut" đến hết clip.
  - Chừa 6 frame trước câu đầu và 6 frame sau câu cuối để editor còn chỗ chỉnh.
- Take mở đầu bằng hành động không lời: giữ từ khoảng 0,5 giây sau tiếng "action". Xem khung hình (`render_frame` với `seconds`) để chắc diễn viên đã vào vai.
- Không bao giờ đưa "ok", "cut", "cắt", "được", "lại", "dừng" vào `transcript.removeFillers`. Lệnh đó xóa mọi chỗ có từ đó, kể cả trong lời thoại. Xác định từng chỗ một, rồi bỏ bằng `transcript.extract` (hoặc `transcript.lift` nếu cần giữ khoảng trống).

### Phân biệt tiếng hô của đạo diễn với lời thoại

"ok", "được rồi", "cắt", "dừng", "lại"… cũng có thể là lời của diễn viên trong kịch bản. Xét lần lượt:

1. **Người nói.** Trước hết tìm những chỗ chắc chắn là đạo diễn: "action" ở đầu take và "cut" ở cuối take. Người nói (`speaker`) của các từ đó là đạo diễn.
   - Một "ok" cùng người nói với đạo diễn thì là tiếng hô.
   - Một "ok" cùng người nói với các câu thoại xung quanh thì là lời diễn viên.
   - Nhãn người nói tự động có thể sai khi hai giọng giống nhau, nên luôn xét thêm các dấu hiệu bên dưới.
2. **Vị trí.**
   - Tiếng hô của đạo diễn thường nằm ngay sau "cut", ở cuối clip hoặc trước "action". Nó hay đứng một mình sau khoảng lặng từ 0,5 giây trở lên, hoặc đi kèm "lại", "lần nữa", "được rồi", "tốt".
   - Lời thoại nằm giữa "action" và "cut", dính liền với câu trước hoặc câu sau, không có khoảng lặng dài.
3. **Kịch bản.** Người dùng có đưa kịch bản và câu thoại có chữ đó thì đó là lời diễn viên.
4. **Hình.** Xem khung hình tại thời điểm của từ đó.
   - Diễn viên đang nói, đang trong vai: lời thoại.
   - Diễn viên thả lỏng, ra khỏi vai, quay ra ngoài hoặc nhìn về phía đạo diễn: tiếng hô.
5. **Độ tin cậy.** Đạo diễn thường đứng xa mic, giọng nhỏ, nên `confidence` hay thấp. Đây chỉ là dấu hiệu phụ.

Vẫn không chắc thì KHÔNG cắt. Đặt marker (`markers.add`, `"color": "Yellow"`) tại từ đó, tên "Kiểm tra: tiếng hô hay lời thoại?", rồi ghi vào báo cáo cuối.

## 3. Chọn take

- Take đạt: sau "cut", đạo diễn khen ("ok", "được rồi", "tốt", "đẹp", "qua").
- Take hỏng: đạo diễn bảo "lại", "làm lại", "lần nữa", hoặc take bị ngắt giữa chừng.
- Nhiều take cho cùng một đoạn:
  - Giữ take đạt. Không có tín hiệu nào thì giữ take trọn vẹn cuối cùng.
  - Không xóa các take khác khỏi project, chỉ không đưa vào rough cut.
  - Hai take ngang nhau: đặt marker Yellow ở take đã chọn, tên "Take khác: <tên clip>".
- Ghi vào báo cáo đã chọn take nào và vì sao.

## 4. Nói trước camera

Chỉ áp dụng cho phỏng vấn, vlog… Với cảnh diễn, khoảng lặng và tiếng "ừ", "à" của diễn viên là diễn xuất, không được bỏ.

- **Khoảng lặng:** bỏ khoảng lặng dài hơn 0,6 giây, chừa lại 0,15 giây: `transcript.removePauses` với `{"minSeconds": 0.6, "keepSeconds": 0.15}`.
- **Từ đệm:** `transcript.removeFillers` với `{"fillers": ["ừm", "ờm", "ờ", "ơ", "um", "uh", "uhm", "ah"]}`.
  - Danh sách có sẵn của app chỉ có từ tiếng Anh.
  - Không đưa "ừ", "à", "kiểu", "thì" vào danh sách: các từ này thường có nghĩa ("Ừ, được"). Chỗ nào rõ là ấp úng thì bỏ riêng chỗ đó.
  - Whisper hay bỏ sót tiếng ấp úng, nên không thấy từ đệm trong transcript không có nghĩa là không có.
- **Câu nói vấp, nói lại:** giữ lần nói trọn câu cuối cùng.

## 5. Sắp xếp track

- **V1, A1–A2:** hình và tiếng chính.
- **V2:** B-roll, cutaway.
- **A3 trở lên:** nhạc, hiệu ứng âm thanh.
- Cắt xong thì đóng khoảng trống (ripple), trừ khi người dùng muốn giữ chỗ trống.

## 6. Hiệu ứng ở giai đoạn rough cut

Bản dựng này sẽ sang Premiere Pro để editor hoàn thiện.

- **Premiere XML mang theo được:** cắt ghép, track, marker, nhãn màu clip, chuyển cảnh kiểu Cross Dissolve, Scale/Position/Rotation (Motion), Opacity, tốc độ (Time Remap), Audio Levels, Pan.
- **Bị mất hoặc không dùng được trong Premiere:** chỉnh màu (Lumetri), title và các hiệu ứng khác.

Cách làm:

- Không chỉnh màu, không thêm title, không dùng hiệu ứng ngoài danh sách mang theo được, trừ khi người dùng yêu cầu. Chỗ cần title hoặc hiệu ứng thì đặt marker tên "TITLE: <nội dung>" hoặc "FX: <mô tả>".
- **Chuyển cảnh:** mặc định cắt thẳng. Chỉ dùng Cross Dissolve (hình) và Constant Power (tiếng) để làm mềm chỗ nối, dài 8 frame. Không dùng chuyển cảnh màu mè.
- **Âm thanh:** nhạc nền nằm dưới thoại thì giảm khoảng 18 dB so với mức gốc của nhạc (`clip.audioGain` hoặc Audio Levels). Không normalize hay mix kỹ, để editor làm ở Premiere.

## 7. Giao bản dựng

- Lưu project (`file.save`, hoặc `file.saveAs` nếu chưa có tên) vào thư mục người dùng chỉ định.
- Xuất Premiere XML (`file.exportPremiere`) cùng thư mục, cùng tên với project.
- Báo cáo ngắn cho người dùng:
  - đường dẫn hai file;
  - đã cắt những gì;
  - đã chọn take nào và vì sao;
  - danh sách marker Yellow cần người kiểm tra.
