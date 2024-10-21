import { ChainInfo } from '@keplr-wallet/types'

// Neutron local chain definition
export const localNeutron: ChainInfo = {
  chainId: 'testing',
  chainName: 'Local Neutron Testchain',
  rpc: 'http://localhost:26657',
  rest: 'http://localhost:1317',
  stakeCurrency: {
    coinDenom: 'NEUTRON',
    coinMinimalDenom: 'untrn',
    coinDecimals: 6,
    coinGeckoId: 'neutron',
  },
  bip44: {
    coinType: 118,
  },
  bech32Config: {
    bech32PrefixAccAddr: 'neutron',
    bech32PrefixAccPub: 'neutron' + 'pub',
    bech32PrefixValAddr: 'neutron' + 'valoper',
    bech32PrefixValPub: 'neutron' + 'valoperpub',
    bech32PrefixConsAddr: 'neutron' + 'valcons',
    bech32PrefixConsPub: 'neutron' + 'valconspub',
  },
  currencies: [
    {
      coinDenom: 'NTRN',
      coinMinimalDenom: 'untrn',
      coinDecimals: 6,
      coinGeckoId: 'neutron',
    },
  ],
  feeCurrencies: [
    {
      coinDenom: 'NTRN',
      coinMinimalDenom: 'untrn',
      coinDecimals: 6,
      coinGeckoId: 'neutron',
      gasPriceStep: { low: 0.001, average: 0.0025, high: 0.004 },
    },
  ],
}
