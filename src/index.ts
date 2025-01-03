import { Bot, Context, session, SessionFlavor, webhookCallback } from 'grammy';
import Koa from 'koa';
import "dotenv/config";
import { Account, AccountAddress, Aptos, APTOS_COIN, AptosConfig, Ed25519PrivateKey, Network, UserTransactionResponse } from '@aptos-labs/ts-sdk';
import { I18n, I18nFlavor } from '@grammyjs/i18n';


const use_webhook = process.env.USE_WEBHOOK === 'true';


interface SessionData {
    __language_code?: string;
}
interface UserCallData {
    lastCall: Date;
}
type MyContext = Context & SessionFlavor<SessionData> & I18nFlavor;
const db: { [key: number]: UserCallData } = {};
const aptos = new Aptos(new AptosConfig({ network: Network.TESTNET}));

const faucetAccount = Account.fromPrivateKey({privateKey: new Ed25519PrivateKey(process.env.FAUCET_PRIVATE_KEY!)});


const bot = new Bot<MyContext>(process.env.TG_BOT_API_KEY!);

const i18n = new I18n<MyContext>({
    defaultLocale: "en",
    useSession: true, 
    directory: "locales", 
});

bot.use(
    session({
      initial: () => {
        return {};
      },
    }),
);
bot.use(i18n);
// 介绍大家我这是一个 Aptos Testnet 水龙头机器人
bot.command('start', ctx => ctx.reply( ctx.t("start") ));
// 输入 /faucet <address> 可以获得 0.1 个 testnet token 每个小时只能调用一次
bot.command('help', ctx => ctx.reply(ctx.t("help")));
bot.command('faucet', async ctx => {
    // 判断上一次这个用户调用的时间是否超过 1 小时
    const lastCall = db[ctx.from!.id];
    if (lastCall && new Date().getTime() - lastCall.lastCall.getTime() < 3600000) {
        return ctx.reply(ctx.t("faucet.too-frequent"));
    };

    // 调用 Aptos SDK 发送 0.1 个 testnet token

    // 获得地址

    let address = ctx.match.split(' ').at(0);

    if(!address) {
        return await ctx.reply(ctx.t("faucet.no-address"));
    }

    let txn = await aptos.transferCoinTransaction({
        sender: faucetAccount.accountAddress,
        recipient: AccountAddress.from(address),
        coinType: APTOS_COIN,
        amount: 10 * 1000 * 1000
    });

    let simulate_result: Array<UserTransactionResponse> | null = null;
    
    try{
        simulate_result = await aptos.transaction.simulate.simple({
            transaction: txn
        });
    }catch(e) {

    }

    if(simulate_result![0].vm_status !== 'Executed successfully') {
        return await ctx.reply(`Transaction simulation failed!\n${simulate_result![0].vm_status}`);
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
    db[ctx.from!.id] = { lastCall: new Date() };

    await ctx.reply(`Transaction submitted!\n\nYou get 0.1 APT in testnet\n\nTxn Hash: ${submit_result.hash}\n\nExplorer: https://explorer.aptoslabs.com/txn/${submit_result.hash}?network=testnet`);
});
bot.command("language", async (ctx) => {
    if (ctx.match === "") {
      return await ctx.reply(ctx.t("language.specify-a-locale", {locales: i18n.locales.join(", ")}));
    }
  
    // `i18n.locale` 包含所有已注册的地区。
    if (!i18n.locales.includes(ctx.match)) {
      return await ctx.reply(ctx.t("language.invalid-locale", {locales: i18n.locales.join(", ")}));
    }
  
    // `ctx.i18n.getLocale` 返回当前使用的地区。
    if ((await ctx.i18n.getLocale()) === ctx.match) {
      return await ctx.reply(ctx.t("language.already-set", {locale: ctx.match}));
    }
  
    await ctx.i18n.setLocale(ctx.match);
    await ctx.reply(ctx.t("language.language-set", {locale: ctx.match}));
  });


if(use_webhook) {
    const app = new Koa();
    app.use(webhookCallback(bot, 'koa'));
    await bot.api.setWebhook(process.env.WEBHOOK_URL!);
    app.listen(3000); 
}else{
    await bot.api.setMyCommands([
        { command: "start", description: "Start the bot" },
        { command: "help", description: "Show help text" },
        { command: "faucet", description: "Get 0.1 testnet APT token" },
        { command: "language", description: "Set language" },
    ]);
    await bot.start();
}