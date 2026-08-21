start =
    Welcome! I'm the Aptos Testnet Faucet Bot.

    I can send you 0.5 testnet APT once per hour.

    Commands:
    /faucet <address> — request testnet tokens
    /language — change language
    /help — show this help

help =
    Here's how to use me:

    /faucet <address>
    Request 0.5 testnet APT. You can do this once per hour.

    /language
    Change the bot language. Available: { $locales }

    Need help? Send /start.

faucet =
    .too-frequent = Please wait a bit! You can request testnet APT once per hour. Try again later.
    .no-address = Please include your Aptos address. Example: /faucet 0x1234...
    Input /faucet <address> to get 0.5 testnet token. You can only call me once per hour!

language =
    .specify-a-locale = Please choose a language: { $locales }
    .invalid-locale = Sorry, that language is not available. Please choose from: { $locales }
    .already-set = Language is already set to { $localeName }
    .language-set = Language changed to { $localeName }
