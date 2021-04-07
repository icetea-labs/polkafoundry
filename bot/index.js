// Telegram bot link: http://t.me/pkf_test_bot

require('dotenv').config();
const TelegramBot = require('node-telegram-bot-api');
const Web3 = require('web3');
const Redis = require("ioredis");

const TOKEN = process.env.TELEGRAM_TOKEN || '1401222730:AAGv-XwfbfiSYZkV-5zVHBkznYbYlUaWJRc';
const GENESIS_ACCOUNT = process.env.GENESIS_ACCOUNT;
const GENESIS_ACCOUNT_PRIVATE_KEY = process.env.GENESIS_ACCOUNT_PRIVATE_KEY;

// create a bot that uses 'polling' to fetch new updates
const bot = new TelegramBot(TOKEN, { polling: true });

const redis = new Redis(process.env.REDIS_URI);
const web3 = new Web3(process.env.SOCKET_URI);

const second2Time = sec => {
  return new Date(sec * 1000).toISOString().substr(11, 8);
};

const customRequest = async (method, params) => {
  return new Promise((resolve, reject) => {
    web3.currentProvider.send({
        jsonrpc: '2.0',
        id: 1,
        method,
        params,
      }, (error, result) => {
        if (error) reject(`Failed to send custom request (${method} (${params.join(',')})): ${error.message || error.toString()}`);
        else resolve(result);
      });
  });
};

const callWeb3 = async address => {
  const tx = await web3.eth.accounts.signTransaction({
      from: GENESIS_ACCOUNT,
      to: address,
      value: web3.utils.toWei("2", "ether"),
      gasPrice: '0x01',
      gas: '0x100000',
    },
    GENESIS_ACCOUNT_PRIVATE_KEY
  );

  const res = await customRequest('eth_sendRawTransaction', [tx.rawTransaction]);

  const balance = await web3.eth.getBalance(address);
  const balance_genesis = await web3.eth.getBalance(GENESIS_ACCOUNT);

  console.log('balance received account after transfer: ', balance);
  console.log('balance GENESIS_ACCOUNT account: ', balance_genesis);

  return res.result;
};

bot.onText(/\/help/, msg => {
  const chatId = msg.chat.id;
  bot.sendMessage(chatId, "Support commands:\n /faucet to_address");
});

bot.onText(/^\/faucet$/, async (msg, match) => {
  const chatId = msg.chat.id;
  const username = msg.chat.username;

  bot.sendMessage(chatId, `@${username} please enter wallet address. Syntax: /faucet to_address`);
});

bot.onText(/\/faucet (.+)/, async (msg, match) => {
  const chatId = msg.chat.id;
  const to_address = match[1];
  const username = msg.chat.username;

  const cacheKey = `faucet_${to_address}`;

  const ttl = await redis.ttl(cacheKey);
  // if(ttl > 0) return bot.sendMessage(chatId, `@${username} has reached their daily quota. Only request once per day. Please wait after ${second2Time(ttl)}`);

  // call web3
  try {
    const transaction = await callWeb3(to_address);
    if (transaction) {
      await redis.set(cacheKey, '1', 'EX', 86400); // 1 day
      bot.sendMessage(chatId, `Sent @${username} 2 ETH. Extrinsic hash: ${transaction}`);
    } else {
      bot.sendMessage(chatId, `@${username} transaction failed`);
    }
  } catch(err) {
    bot.sendMessage(chatId, `@${username} transaction failed\n` + err);
  }
});
