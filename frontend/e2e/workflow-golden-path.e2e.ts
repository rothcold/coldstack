import { expect, test, type APIRequestContext } from '@playwright/test'

async function createEmployee(
  request: APIRequestContext,
  payload: {
    name: string
    role: string
    workflow_role: 'planner' | 'designer' | 'coder' | 'reviewer' | 'qa'
  },
) {
  const response = await request.post('/api/employees', {
    data: {
      ...payload,
      department: 'Workflow',
      agent_backend: 'claude_code',
    },
  })
  expect(response.ok()).toBeTruthy()
  return response.json()
}

test('golden workflow path reaches archive', async ({ page, request }) => {
  const suffix = Date.now()
  const planner = await createEmployee(request, {
    name: `Planner ${suffix}`,
    role: 'Planner',
    workflow_role: 'planner',
  })
  const designer = await createEmployee(request, {
    name: `Designer ${suffix}`,
    role: 'Designer',
    workflow_role: 'designer',
  })
  const coder = await createEmployee(request, {
    name: `Coder ${suffix}`,
    role: 'Coder',
    workflow_role: 'coder',
  })
  const reviewer = await createEmployee(request, {
    name: `Reviewer ${suffix}`,
    role: 'Reviewer',
    workflow_role: 'reviewer',
  })
  const qa = await createEmployee(request, {
    name: `QA ${suffix}`,
    role: 'QA',
    workflow_role: 'qa',
  })

  const taskId = `T-E2E-${suffix}`
  const createTask = await request.post('/api/tasks', {
    data: {
      task_id: taskId,
      title: `Golden path ${suffix}`,
      description: 'Walk the workflow end to end.',
      assignee: planner.name,
    },
  })
  expect(createTask.ok()).toBeTruthy()

  await page.goto('/')
  await page.getByText(`Golden path ${suffix}`).click()
  const actorSelect = page.getByRole('combobox').first()

  await actorSelect.selectOption(`employee:${planner.id}`)
  await page.getByRole('button', { name: 'Advance to Design' }).click()
  await expect(page.getByText(`${planner.name} moved this task to Design`)).toBeVisible()

  await actorSelect.selectOption(`employee:${designer.id}`)
  await page.getByRole('button', { name: 'Advance to Coding' }).click()
  await expect(page.getByText(`${designer.name} moved this task to Coding`)).toBeVisible()

  await actorSelect.selectOption(`employee:${coder.id}`)
  await page.getByRole('button', { name: 'Advance to Review' }).click()
  await expect(page.getByText(`${coder.name} moved this task to Review`)).toBeVisible()

  await actorSelect.selectOption(`employee:${reviewer.id}`)
  await page.getByRole('button', { name: 'Return to Coding' }).click()
  await page.getByPlaceholder('What still needs to change?').fill('Please fix the review feedback.')
  await page.getByRole('button', { name: 'Return task' }).click()
  await expect(page.getByText(`${reviewer.name} returned this task to Coding`)).toBeVisible()

  await actorSelect.selectOption(`employee:${coder.id}`)
  await page.getByRole('button', { name: 'Advance to Review' }).click()
  await expect(page.getByText(`${coder.name} moved this task to Review`)).toBeVisible()

  await actorSelect.selectOption(`employee:${reviewer.id}`)
  await page.getByRole('button', { name: 'Advance to QA' }).click()
  await expect(page.getByText(`${reviewer.name} moved this task to QA`)).toBeVisible()

  await actorSelect.selectOption(`employee:${qa.id}`)
  await page.getByRole('button', { name: 'Advance to NeedsHuman' }).click()
  await expect(page.getByText(`${qa.name} moved this task to NeedsHuman`)).toBeVisible()

  await actorSelect.selectOption('human:self')
  await page.getByRole('button', { name: 'Approve' }).click()
  await expect(page.getByText('Human decision moved this task to Done')).toBeVisible()

  await actorSelect.selectOption('human:self')
  await page.getByRole('button', { name: 'Approve and archive' }).click()

  await expect(page.locator('button').filter({ hasText: `Golden path ${suffix}` })).toHaveCount(0)
})
