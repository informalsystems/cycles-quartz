import { BrowserContext, Page } from '@playwright/test'
import { signTx } from './signTx'

export const getBalance = async ({
  context,
  page,
}: {
  context: BrowserContext
  page: Page
}) => {
  // Check new balance
  await page.getByRole('button', { name: /get/i }).click()

  await signTx({
    context,
    page,
    notificationMsg: 'Balance updated successfully',
  })

  // Wait for the success alert to appear so we know balance updated
  await page.getByText(/\$/i).waitFor({ state: 'visible' })

  return page.getByText(/\$/i).textContent()
}
