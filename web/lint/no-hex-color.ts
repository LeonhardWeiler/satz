import { defineRule, type ESTree } from 'vite-plus/lint/plugins'

const message = 'Read the color from a CSS variable, as the theme sets it.'
const HEX = /#(?:[\da-f]{3}){1,2}(?![\w-])/i

export default defineRule({
  create: (context) => {
    const check = (node: ESTree.Node, text: unknown) => typeof text === 'string' && HEX.test(text) && context.report({ node, message })
    return {
      Literal: (node) => check(node, node.value),
      TemplateElement: (node) => check(node, node.value.raw),
    }
  },
})
