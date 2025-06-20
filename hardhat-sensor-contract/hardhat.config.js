require("@nomicfoundation/hardhat-toolbox");
require("dotenv").config();

// Private key dari akun #0 yang selalu sama saat menjalankan `npx hardhat node`
const LOCALHOST_PRIVATE_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

const { SEPOLIA_RPC_URL, WALLET_PRIVATE_KEY } = process.env;

/** @type import('hardhat/config').HardhatUserConfig */
module.exports = {
  solidity: "0.8.24",
  networks: {
    localhost: {
      url: "http://127.0.0.1:8545",
      // Akun diambil dari node hardhat
    },
    sepolia: {
      url: SEPOLIA_RPC_URL || "",
      accounts: WALLET_PRIVATE_KEY ? [WALLET_PRIVATE_KEY] : [],
    },
  },
};