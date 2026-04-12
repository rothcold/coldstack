import { expect, test } from '@playwright/test'

test.use({
  viewport: { width: 430, height: 932 },
})

test('mobile workflow opens detail sheet', async ({ page, request }) => {
  const suffix = Date.now()
  const createTask = await request.post('/api/tasks', {
    data: {
      task_id: `T-MOBILE-${suffix}`,
      title: `Mobile sheet ${suffix}`,
      description: 'Open this in the mobile detail sheet.',
    },
  })
  expect(createTask.ok()).toBeTruthy()

  await page.goto('/')
  await page.getByRole('button', { name: 'Plan', exact: true }).click()
  await page.getByText(`Mobile sheet ${suffix}`).click()

  await expect(page.getByRole('dialog', { name: `Mobile sheet ${suffix}` })).toBeVisible()
  await expect(page.getByText('Open this in the mobile detail sheet.')).toBeVisible()
})
