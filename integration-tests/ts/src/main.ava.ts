import { Worker, NearAccount, tGas, NEAR, BN } from 'near-workspaces';
import anyTest, { TestFn } from 'ava';
import { mint_more, nft_total_supply } from './utils';

const test = anyTest as TestFn<{
    worker: Worker;
    accounts: Record<string, NearAccount>;
}>;

test.beforeEach(async t => {
    const worker = await Worker.init();
    const root = worker.rootAccount;
    const nft = await root.devDeploy(
        '../../res/non_fungible_token.wasm',
        {
            initialBalance: NEAR.parse('100 N').toJSON(),
            method: "new",
            args: { owner_id: root, metadata: { spec: "nft-1.0.0", name: "Tonic Greedy Goblins", symbol: "GGB" } }
        },
    );
    await root.call(
        nft,
        "nft_mint",
        {
            token_id: "0",
            receiver_id: root,
            token_metadata: {
                title: "Olympus Mons",
                description: "The tallest mountain in the charted solar system",
                media: null,
                media_hash: null,
                copies: 10000,
                issued_at: null,
                expires_at: null,
                starts_at: null,
                updated_at: null,
                extra: null,
                reference: null,
                reference_hash: null,
            }
        },
        { attachedDeposit: '7000000000000000000000' }
    );

    const alice = await root.createSubAccount('alice', { initialBalance: NEAR.parse('100 N').toJSON() });

    t.context.worker = worker;
    t.context.accounts = { root, alice, nft };
});

test.afterEach(async t => {
    await t.context.worker.tearDown().catch(error => {
        console.log('Failed to tear down the worker:', error);
    });
});

test('Approved account transfers token', async test => {
    const { root, alice, nft } = test.context.accounts;
    await root.call(
        nft,
        'nft_approve',
        {
            token_id: '0',
            account_id: alice,

        },
        { attachedDeposit: new BN('270000000000000000000'), gas: tGas('150') },
    );

    await alice.call(
        nft,
        'nft_transfer',
        {
            receiver_id: alice,
            token_id: '0',
            approval_id: 1,
            memo: 'gotcha! bahahaha',
        },
        { attachedDeposit: '1', gas: tGas('150') }
    );

    const token: any = await nft.view('nft_token', { token_id: '0' });
    test.is(token.owner_id, alice.accountId);
});

test('Simple transfer', async test => {
    const { root, alice, nft } = test.context.accounts;
    let token: any = await nft.view('nft_token', { token_id: '0' });
    test.is(token.owner_id, root.accountId);

    const result = await root.callRaw(
        nft,
        'nft_transfer',
        {
            receiver_id: alice,
            token_id: '0',
            memo: "simple transfer",
        },
        { attachedDeposit: '1' },
    );
    test.assert(result.succeeded);
    token = await nft.view('nft_token', { token_id: '0' });
    test.is(token.owner_id, alice.accountId);
});

test('Enum total supply', async test => {
    const { root, alice, nft } = test.context.accounts;
    await mint_more(root, nft);

    const total_supply = await nft_total_supply(nft, alice);
    test.deepEqual(total_supply, new BN(4));
});

test('Enum nft tokens', async test => {
    const { root, nft } = test.context.accounts;
    await mint_more(root, nft);

    // No optional args should return all
    let tokens: any[] = await nft.view('nft_tokens');
    test.is(tokens.length, 4);

    // Start at "1", with no limit arg
    tokens = await nft.view('nft_tokens', { from_index: '1' });
    test.is(tokens.length, 3);
    test.is(tokens[0].token_id, '1');
    test.is(tokens[1].token_id, '2');
    test.is(tokens[2].token_id, '3');

    // Start at "2", with limit 1
    tokens = await nft.view('nft_tokens', { from_index: '2', limit: 1 });
    test.is(tokens.length, 1);
    test.is(tokens[0].token_id, '2');

    // Don't specify from_index, but limit 2
    tokens = await nft.view('nft_tokens', { limit: 2 });
    test.is(tokens.length, 2);
    test.is(tokens[0].token_id, '0');
    test.is(tokens[1].token_id, '1');
});

test('Enum nft supply for owner', async test => {
    const { root, alice, nft } = test.context.accounts;
    // Get number from account with no NFTs
    let ownerNumTokens: BN = new BN(await nft.view('nft_supply_for_owner', { account_id: alice }));
    test.deepEqual(ownerNumTokens, new BN(0));

    ownerNumTokens = new BN(await nft.view('nft_supply_for_owner', { account_id: root }));
    test.deepEqual(ownerNumTokens, new BN(1));

    await mint_more(root, nft);

    ownerNumTokens = new BN(await nft.view('nft_supply_for_owner', { account_id: root }));
    test.deepEqual(ownerNumTokens, new BN(4));
});

test('Enum nft tokens for owner', async test => {
    const { root, alice, nft } = test.context.accounts;
    await mint_more(root, nft);

    // Get tokens from account with no NFTs
    let ownerTokens: any[] = await nft.view('nft_tokens_for_owner', { account_id: alice });
    test.deepEqual(ownerTokens.length, 0);

    // Get tokens with no optional args
    ownerTokens = await nft.view('nft_tokens_for_owner', { account_id: root });
    test.deepEqual(ownerTokens.length, 4);

    // With from_index and no limit
    ownerTokens = await nft.view('nft_tokens_for_owner', { account_id: root, from_index: new BN(2) });
    test.deepEqual(ownerTokens.length, 2);
    test.is(ownerTokens[0].token_id, '2');
    test.is(ownerTokens[1].token_id, '3');

    // With from_index and limit 1
    ownerTokens = await nft.view('nft_tokens_for_owner', { account_id: root, from_index: new BN(1), limit: 1 });
    test.deepEqual(ownerTokens.length, 1);
    test.is(ownerTokens[0].token_id, '1');

    // No from_index but limit 3
    ownerTokens = await nft.view('nft_tokens_for_owner', { account_id: root, limit: 3 });
    test.deepEqual(ownerTokens.length, 3);
    test.is(ownerTokens[0].token_id, '0');
    test.is(ownerTokens[1].token_id, '1');
    test.is(ownerTokens[2].token_id, '2');
});