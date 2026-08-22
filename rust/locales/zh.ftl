start =
    欢迎！我是 Aptos 测试网水龙头机器人。

    我可以每小时向你发送 0.5 个测试网 APT。

    命令：
    /faucet \<地址\> — 领取测试网代币
    /language — 切换语言
    /help — 显示帮助

help =
    使用方法：

    /faucet \<地址\>
    领取 0.5 个测试网 APT。每小时只能领取一次。

    /language
    切换机器人语言。你可以使用：
    { $languageCommands }

    需要帮助？发送 /start。

faucet =
    .too-frequent = 请稍等一下！每小时只能领取一次测试网 APT，稍后再试。
    .no-address = 请提供你的 Aptos 地址。例如：/faucet 0x1234...
    输入 /faucet \<地址\> 以获取 0.5 测试网代币。你每小时只能调用一次！

language =
    .specify-a-locale = 请选择语言：
    { $languageCommands }
    .invalid-locale = 抱歉，该语言不可用。请从以下中选择：
    { $languageCommands }
    .already-set = 当前语言已设置为 { $localeName }
    .language-set = 语言已切换为 { $localeName }
