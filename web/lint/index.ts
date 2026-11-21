import { definePlugin } from 'vite-plus/lint/plugins'
import noCachedRejection from './no-cached-rejection.ts'
import noHexColor from './no-hex-color.ts'
import noTestTiming from './no-test-timing.ts'
import noUnitText from './no-unit-text.ts'
import wayOut from './way-out.ts'

export default definePlugin({
  meta: { name: 'satz' },
  rules: { 'no-cached-rejection': noCachedRejection, 'no-hex-color': noHexColor, 'no-test-timing': noTestTiming, 'no-unit-text': noUnitText, 'way-out': wayOut },
})
