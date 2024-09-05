import test from './fixtures'
import { connectWallet } from './helpers/connectWalet'
import { setSeedPhrase } from './helpers/setSeedPhrase'

test.beforeEach(async ({ context, page }) => {
  await connectWallet({ context, page })
})

test.describe('Seed Phrase', () => {
  test('can use autogenerated seed phrase', async ({ page }) => {
    await setSeedPhrase({ page })
    await test
      .expect(
        await page.evaluate(() =>
          window.localStorage.getItem('ephemeral-mnemonic'),
        ),
      )
      .toBeDefined()
  })

  test('can enter and use a custom seed phrase', async ({ page }) => {
    await setSeedPhrase({ page, seedPhrase: process.env.TEST_WALLET_MNEMONIC! })
    await test
      .expect(
        await page.evaluate(() =>
          window.localStorage.getItem('ephemeral-mnemonic'),
        ),
      )
      .toEqual(process.env.TEST_WALLET_MNEMONIC)
  })
})
