start =
    Chào mừng! Tôi là Bot Vòi Testnet Aptos.

    Tôi có thể gửi cho bạn 0.5 testnet APT mỗi giờ một lần.

    Lệnh:
    /faucet <địa chỉ> — yêu cầu token testnet
    /language — thay đổi ngôn ngữ
    /help — hiển thị trợ giúp

help =
    Cách sử dụng tôi:

    /faucet <địa chỉ>
    Yêu cầu 0.5 testnet APT. Bạn chỉ có thể thực hiện mỗi giờ một lần.

    /language
    Thay đổi ngôn ngữ bot. Các ngôn ngữ khả dụng: { $locales }

    Cần trợ giúp? Gửi /start.

faucet =
    .too-frequent = Vui lòng đợi một chút! Bạn chỉ có thể yêu cầu testnet APT mỗi giờ một lần. Hãy thử lại sau.
    .no-address = Vui lòng cung cấp địa chỉ Aptos của bạn. Ví dụ: /faucet 0x1234...
    Nhập /faucet <địa chỉ> để nhận 0.5 token testnet. Bạn chỉ có thể gọi tôi một lần mỗi giờ!

language =
    .specify-a-locale = Vui lòng chọn ngôn ngữ: { $locales }
    .invalid-locale = Xin lỗi, ngôn ngữ đó không khả dụng. Vui lòng chọn từ: { $locales }
    .already-set = Ngôn ngữ hiện tại đã được đặt thành { $localeName }
    .language-set = Ngôn ngữ đã được thay đổi thành { $localeName }
