import { Bot, Context, InlineKeyboard, session, SessionFlavor, webhookCallback } from 'grammy';
import Koa from 'koa';
import "dotenv/config";
import { Account, AccountAddress, Aptos, APTOS_COIN, AptosConfig, Ed25519PrivateKey, Network, UserTransactionResponse } from '@aptos-labs/ts-sdk';
import { I18n, I18nFlavor } from '@grammyjs/i18n';
import { getLastCall, setLastCall, sqliteSessionStorage } from './db.js';


const use_webhook = process.env.USE_WEBHOOK === 'true';

if (!process.env.TG_BOT_API_KEY) {
    throw new Error('TG_BOT_API_KEY is required');
}
if (!process.env.FAUCET_PRIVATE_KEY) {
    throw new Error('FAUCET_PRIVATE_KEY is required');
}
if (use_webhook && !process.env.WEBHOOK_URL) {
    throw new Error('WEBHOOK_URL is required when USE_WEBHOOK=true');
}

interface SessionData {
    __language_code?: string;
}
type MyContext = Context & SessionFlavor<SessionData> & I18nFlavor;
const aptos = new Aptos(new AptosConfig({ network: Network.TESTNET}));

const faucetAccount = Account.fromPrivateKey({privateKey: new Ed25519PrivateKey(process.env.FAUCET_PRIVATE_KEY)});


const bot = new Bot<MyContext>(process.env.TG_BOT_API_KEY);

const i18n = new I18n<MyContext>({
    defaultLocale: "en",
    useSession: true, 
    directory: "locales", 
});

bot.use(
    session({
      initial: () => ({
        __language_code: "en",
      }),
      storage: sqliteSessionStorage<SessionData>(),
    }),
);
bot.use(i18n);

const LOCALE_NAMES: Record<string, string> = {
    en: "English",
    vi: "Tiếng Việt",
    zh: "中文",
};

function formatLanguageCommands(locales: string[]): string {
    const commands = locales.map((locale) => `/language ${locale}`).join("\n");
    return "```\n" + commands + "\n```";
}

// 介绍大家我这是一个 Aptos Testnet 水龙头机器人
bot.command('start', ctx => { 
    if(ctx.chat?.type != 'private') {
        return 
    }
    return ctx.reply( ctx.t("start"))
});
// 输入 /faucet <address> 可以获得 0.5 个 testnet token 每个小时只能调用一次
bot.command('help', async ctx => {
    let is_private = ctx.chat?.type === 'private';
    const helpText = ctx.t("help", { languageCommands: formatLanguageCommands(i18n.locales) });
    if(is_private) {
        return await ctx.reply(helpText, { parse_mode: "Markdown" });
    }else{
        let message = await ctx.reply(helpText, { parse_mode: "Markdown", message_thread_id: ctx.message?.message_thread_id });
        return deleteMessage(message.chat.id, message.message_id);
    }

});
bot.command('faucet', async ctx => {

    let is_thread = ctx.message?.is_topic_message;
    let is_private = ctx.chat?.type === 'private';

    // 判断上一次这个用户调用的时间是否超过 1 小时
    const lastCall = getLastCall(ctx.from!.id);
    if (lastCall && new Date().getTime() - lastCall.getTime() < 3600000) {
        let message = await ctx.reply(ctx.t("faucet.too-frequent"),{message_thread_id: is_thread ? ctx.message?.message_thread_id : undefined});
        deleteMessage(message.chat.id, message.message_id);
        return 
    };



    // 调用 Aptos SDK 发送 0.5 个 testnet token

    // 获得地址

    let address = ctx.match.split(' ').at(0);

    if(!address) {
        let message = await ctx.reply(ctx.t("faucet.no-address"));
        deleteMessage(message.chat.id, message.message_id);
        return 
    }

    let txn = await aptos.transferCoinTransaction({
        sender: faucetAccount.accountAddress,
        recipient: AccountAddress.from(address),
        coinType: APTOS_COIN,
        amount: 50 * 1000 * 1000
    });

    let simulate_result: Array<UserTransactionResponse> | null = null;
    
    try{
        simulate_result = await aptos.transaction.simulate.simple({
            transaction: txn
        });
    }catch(e) {

    }

    if(!simulate_result || simulate_result[0].vm_status !== 'Executed successfully') {
        const vmStatus = simulate_result ? simulate_result[0].vm_status : 'Simulation request failed';
        let message = await ctx.reply(`Transaction simulation failed!\n${vmStatus}`,{message_thread_id: is_thread ? ctx.message?.message_thread_id : undefined});
        deleteMessage(message.chat.id, message.message_id);
        return  
    }

    let faucetAccountAuth = aptos.transaction.sign({
        transaction: txn,
        signer: faucetAccount
    });

    let submit_result = await aptos.transaction.submit.simple({
        transaction: txn,
        senderAuthenticator:faucetAccountAuth
    });

    await aptos.waitForTransaction({transactionHash: submit_result.hash});

    // 更新用户调用时间
    setLastCall(ctx.from!.id, new Date());

    // 制作一个按钮，可以由某个人删除信息
    const keyboard = new InlineKeyboard().text(
        `Delete this message`,
        `delete_${ctx.from?.id}`
    );

    await ctx.reply(`Transaction submitted!\n\nYou get 0.5 APT in testnet\n\nTxn Hash: ${submit_result.hash}\n\nExplorer: https://explorer.aptoslabs.com/txn/${submit_result.hash}?network=testnet`, { reply_markup: is_private ? undefined : keyboard, message_thread_id: is_thread ? ctx.message?.message_thread_id : undefined});
    await bot.api.setMessageReaction( ctx.chat!.id, ctx.message!.message_id , [{type: "emoji", emoji:'👌'}],);
});
bot.command("language", async (ctx) => {
    if(ctx.chat?.type != 'private') {
        return 
    }
    const languageCommands = formatLanguageCommands(i18n.locales);
    if (ctx.match === "") {
      return await ctx.reply(ctx.t("language.specify-a-locale", {languageCommands}), { parse_mode: "Markdown" });
    }
  
    // `i18n.locale` 包含所有已注册的地区。
    if (!i18n.locales.includes(ctx.match)) {
      return await ctx.reply(ctx.t("language.invalid-locale", {languageCommands}), { parse_mode: "Markdown" });
    }
  
    const localeName = LOCALE_NAMES[ctx.match] ?? ctx.match;
    // `ctx.i18n.getLocale` 返回当前使用的地区。
    if ((await ctx.i18n.getLocale()) === ctx.match) {
      return await ctx.reply(ctx.t("language.already-set", {localeName}));
    }
  
    await ctx.i18n.setLocale(ctx.match);
    await ctx.reply(ctx.t("language.language-set", {localeName}));
  });
bot.callbackQuery(/^delete_/, async (ctx) => {
    if (!ctx.callbackQuery?.data) return;
    let [id] = ctx.callbackQuery.data.split('_').splice(1); 
    await ctx.answerCallbackQuery();
    if (ctx.from?.id?.toString() === id) {
      await ctx.api.deleteMessage(ctx.chat!.id, ctx.callbackQuery!.message!.message_id);
    }
});

bot.catch((e) => {
    console.error(`Error for `, e);
  }
);


if(use_webhook) {
    // 启动时丢弃在离线期间堆积的旧 update，只处理开机之后的消息
    await bot.api.deleteWebhook({ drop_pending_updates: true });
    const app = new Koa();
    app.use(webhookCallback(bot, 'koa'));
    app.listen(8000);
    await bot.api.setWebhook( process.env.WEBHOOK_URL! );
    
}else{
    // 启动时丢弃在离线期间堆积的旧 update，只处理开机之后的消息
    await bot.start({ drop_pending_updates: true });
}

await bot.api.setMyCommands([
    { command: "start", description: "Start the bot" },
    { command: "help", description: "Show help text" },
    { command: "faucet", description: "Get 0.5 testnet APT token" },
    { command: "language", description: "Set language" },
]);
 
export function deleteMessage(chat_id: number, message_id: number, time: number = 5) {
    setTimeout(async () => {
      try {
        await bot.api.deleteMessage(chat_id, message_id);
      } catch (e) {
        console.error('Failed to delete message:', e);
      }
    }, 1000 * time);
  }