import * as anchor from '@project-serum/anchor';
import {
  PublicKey
} from '@solana/web3.js';
import { Todo } from "../target/types/todo";

const expect = require('chai').expect;
const { SystemProgram, LAMPORTS_PER_SOL } = anchor.web3;

const provider = anchor.Provider.env();
anchor.setProvider(provider);
const mainProgram: anchor.Program<Todo> = anchor.workspace.Todo;


async function getAccountBalance(pubkey) {
  let account = await provider.connection.getAccountInfo(pubkey);
  return account?.lamports ?? 0;
}

function expectBalance(actual, expected, message, slack = 20000) {
  expect(actual, message).within(expected - slack, expected + slack);
}

async function createUser(airdropBalance?) {
  airdropBalance = airdropBalance ?? 10 * LAMPORTS_PER_SOL;
  let user = anchor.web3.Keypair.generate();
  let sig = await provider.connection.requestAirdrop(user.publicKey, airdropBalance);
  await provider.connection.confirmTransaction(sig);

  let wallet = new anchor.Wallet(user);
  let userProvider = new anchor.Provider(provider.connection, wallet, provider.opts);

  return {
    key: user,
    wallet,
    provider: userProvider,
  };
}

function createUsers(numUsers) {
  let promises = [];
  for (let i = 0; i < numUsers; i++) {
    promises.push(createUser());
  }

  return Promise.all(promises);
}

function programForUser(user) {
  return new anchor.Program(mainProgram.idl, mainProgram.programId, user.provider);
}

async function createList(owner: {
  key: anchor.web3.Keypair,
  wallet: anchor.Wallet,
  provider: anchor.Provider
}, name, capacity = 16) {
  const [listAccount, bump] = await PublicKey.findProgramAddress(
    ['todolist', owner.key.publicKey.toBuffer(), name.slice(0, 32)],
    mainProgram.programId
  );

  // let program = programForUser(owner);
  await mainProgram.rpc.newList(name, capacity, bump, {
    accounts: {
      list: listAccount,
      user: owner.key.publicKey,
      systemProgram: SystemProgram.programId,
    },
    signers: [
      owner.key
    ]
  });

  let list = await mainProgram.account.todoList.fetch(listAccount);
  return { publicKey: listAccount, data: list };
}

describe('new list', () => {
  it('creates a list', async () => {
    const owner = await createUser();
    let list = await createList(owner, 'A list');

    expect(list.data.listOwner.toString(), 'List owner is set').equals(owner.key.publicKey.toString());
    expect(list.data.name, 'List name is set').equals('A list');
    expect(list.data.lines.length, 'List has no items').equals(0);
  })
})