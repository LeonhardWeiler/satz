import { defineRule } from 'vite-plus/lint/plugins'
import { called } from './match.ts'

const message = 'Wait for a state, not for time; a timeout belongs in playwright.config.ts.'

export default defineRule({
  create: (context) => ({
    CallExpression(node) {
      const c = node.callee
      if (called(c, 'setTimeout') || called(c, 'waitForTimeout') || called(c, 'slow', 'test')) context.report({ node, message })
    },
    Property(node) {
      const o = node.parent
      if (node.key.type === 'Identifier' && node.key.name === 'timeout' && o.type === 'ObjectExpression' && o.parent.type === 'CallExpression' && o.parent.arguments.includes(o)) context.report({ node, message })
    },
  }),
})
