
// Telegram bot link: http://t.me/pkf_test_bot

const TelegramBot = require('node-telegram-bot-api');
const Web3 = require('web3');
const { RPC_PORT, WS_PORT, GENESIS_ACCOUNT, GENESIS_ACCOUNT_PRIVATE_KEY } = require('../tests/test/constants');

const { createAndFinalizeBlock, customRequest } = require('../tests/test/utils');

const token = '1401222730:AAGv-XwfbfiSYZkV-5zVHBkznYbYlUaWJRc';

// create a bot that uses 'polling' to fetch new updates
const bot = new TelegramBot(token, { polling: true });

let web3;
let balance;
let balance_genesis;
let res;
async function callWeb3(address) {
  try {
    // web3 = new Web3(`ws://localhost:9944`);
    web3 = new Web3(`ws://54.169.215.160:9944`);
  }
  catch (err) {
    return [null, err];
  }

  let tx;
  try {
    tx = await web3.eth.accounts.signTransaction(
      {
        from: GENESIS_ACCOUNT,
        to: address,
        value: web3.utils.toWei("1", "ether"),
        gasPrice: '0x01',
        gas: '0x100000',
      },
      GENESIS_ACCOUNT_PRIVATE_KEY
    );
    console.log('address: ', address);
  } catch (err) {
    return [null, err];
  }

  try {
    res = await customRequest(web3, 'eth_sendRawTransaction', [tx.rawTransaction]);
  } catch (err) {
    return [null, err];
  }

  balance = await web3.eth.getBalance(address);
  balance_genesis = await web3.eth.getBalance(GENESIS_ACCOUNT)
  console.log('balance received account after transfer: ', balance);
  console.log('balance GENESIS_ACCOUNT account: ', balance_genesis)
  return [res.result, null];
};

bot.onText(/\/help/, (msg, match) => {
  const chatId = msg.chat.id;
  bot.sendMessage(chatId, "Support commands:\n /faucet to_address");
});


bot.onText(/\/faucet (.+)/, async (msg, match) => {
  const chatId = msg.chat.id;
  const to_address = match[1];

  // call web3
  const [transaction, err] = await callWeb3(to_address);

  // send back chat
  if (transaction != null)
    bot.sendMessage(chatId, "Transaction successful with code:\n" + transaction);
  else
    bot.sendMessage(chatId, "Transaction failed\n" + err);
});