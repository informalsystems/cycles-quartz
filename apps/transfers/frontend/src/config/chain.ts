import { ChainInfo } from '@keplr-wallet/types'

import { localWasm } from './chains/localWasm'
import { localNeutron } from './chains/localNeutron'
import { testnetNeutron } from './chains/testnetNeutron'


const supportedChains: Record<string, ChainInfo> = {
  doWasm: {
    ...localWasm,
    chainName: 'Digital Ocean Testchain',
    rpc: 'http://143.244.186.205:26657',
    rest: 'http://143.244.186.205:1317',
  },
  localNeutron,
  localWasm,
  testnetNeutron,
}

const chain = supportedChains[process.env.NEXT_PUBLIC_TARGET_CHAIN!]

export default chain
