import { BrowserContext, Page } from '@playwright/test'

export const signTx = async ({
  context,
  page,
  notificationMsg = /successfully/i,
}: {
  context: BrowserContext
  page: Page
  notificationMsg?: string | RegExp
}) => {
  // Sign tx
  const signPage = await context.waitForEvent('page')

  await signPage.getByRole('button', { name: /approve/i }).click()
  await signPage.waitForEvent('close')
  await page.getByText(notificationMsg).waitFor({ state: 'visible' })
}
