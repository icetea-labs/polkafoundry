
// Telegram bot link: http://t.me/pkf_test_bot

const TelegramBot = require('node-telegram-bot-api');
const Web3 = require('web3');
const { RPC_PORT, WS_PORT, GENESIS_ACCOUNT, GENESIS_ACCOUNT_PRIVATE_KEY } = require('../tests/test/constants');

const { createAndFinalizeBlock, customRequest } = require('../tests/test/utils');

const token = '1401222730:AAGv-XwfbfiSYZkV-5zVHBkznYbYlUaWJRc';

// create a bot that uses 'polling' to fetch new updates
const bot = new TelegramBot(token, { polling: true });

let web3;

async function callWeb3(address) {
  try {
    web3 = new Web3(`ws://localhost:${WS_PORT}`);
  }
  catch (err) {
    console.log(err);
  }

  const tx = await web3.eth.accounts.signTransaction(
    {
      from: GENESIS_ACCOUNT,
      to: address,
      value: '0x10',
      gasPrice: '0x01',
      gas: '0x100000',
    },
    GENESIS_ACCOUNT_PRIVATE_KEY
  );
  await customRequest(web3, 'eth_sendRawTransaction', [tx.rawTransaction]);
  // await createAndFinalizeBlock(web3);
};

bot.onText(/\/help/, (msg, match) => {
  const chatId = msg.chat.id;
  bot.sendMessage(chatId, "Support commands: faucet to_address");
});


bot.onText(/\/faucet (.+)/, async (msg, match) => {
  const chatId = msg.chat.id;
  const to_address = match[1];

  // call web3
  ret = await callWeb3(to_address);

  // send back chat
  bot.sendMessage(chatId, "Call: " + to_address);
});