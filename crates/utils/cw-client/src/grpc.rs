use std::error::Error;

use anyhow::anyhow;
use cosmos_sdk_proto::{
    cosmos::{
        auth::v1beta1::{
            query_client::QueryClient as AuthQueryClient, BaseAccount as RawBaseAccount,
            QueryAccountRequest,
        },
        tx::v1beta1::{
            service_client::ServiceClient, BroadcastMode, BroadcastTxRequest, BroadcastTxResponse,
        },
    },
    cosmwasm::wasm::v1::{
        query_client::QueryClient as WasmdQueryClient, QueryRawContractStateRequest,
        QuerySmartContractStateRequest,
    },
    traits::Message,
    Any,
};
use cosmrs::{
    auth::BaseAccount,
    cosmwasm::MsgExecuteContract,
    crypto::{secp256k1::SigningKey, PublicKey},
    tendermint::chain::Id as TmChainId,
    tx,
    tx::{Fee, Msg, SignDoc, SignerInfo},
    AccountId, Coin,
};
use reqwest::Url;
use serde::de::DeserializeOwned;

use crate::CwClient;

pub struct GrpcClient {
    sk: SigningKey,
    url: Url,
}

impl GrpcClient {
    pub fn new(sk: SigningKey, url: Url) -> Self {
        Self { sk, url }
    }
}

#[async_trait::async_trait]
impl CwClient for GrpcClient {
    type Address = AccountId;
    type Query = serde_json::Value;
    type RawQuery = String;
    type ChainId = TmChainId;
    type Error = anyhow::Error;

    async fn query_smart<R: DeserializeOwned + Send>(
        &self,
        contract: &Self::Address,
        query: Self::Query,
    ) -> Result<R, Self::Error> {
        let mut client = WasmdQueryClient::connect(self.url.to_string()).await?;

        let raw_query_request = QuerySmartContractStateRequest {
            address: contract.to_string(),
            query_data: query.to_string().into_bytes(),
        };

        let raw_query_response = client.smart_contract_state(raw_query_request).await?;

        let raw_value = raw_query_response.into_inner().data;
        serde_json::from_slice(&raw_value)
            .map_err(|e| anyhow!("failed to deserialize JSON reponse: {}", e))
    }

    async fn query_raw<R: DeserializeOwned + Default>(
        &self,
        contract: &Self::Address,
        query: Self::RawQuery,
    ) -> Result<R, Self::Error> {
        let mut client = WasmdQueryClient::connect(self.url.to_string()).await?;

        let raw_query_request = QueryRawContractStateRequest {
            address: contract.to_string(),
            query_data: query.to_string().into_bytes(),
        };

        let raw_query_response = client.raw_contract_state(raw_query_request).await?;

        let raw_value = raw_query_response.into_inner().data;
        serde_json::from_slice(&raw_value)
            .map_err(|e| anyhow!("failed to deserialize JSON reponse: {}", e))
    }

    fn query_tx<R: DeserializeOwned + Default>(&self, _txhash: &str) -> Result<R, Self::Error> {
        unimplemented!()
    }

    async fn tx_execute<M: ToString + Send>(
        &self,
        contract: &Self::Address,
        chain_id: &TmChainId,
        gas: u64,
        _sender: &str,
        msg: M,
        _fees: &str,
    ) -> Result<String, Self::Error> {
        let tm_pubkey = self.sk.public_key();
        let sender = tm_pubkey
            .account_id("neutron")
            .map_err(|e| anyhow!("failed to create AccountId from pubkey: {}", e))?;

        let msgs = vec![MsgExecuteContract {
            sender: sender.clone(),
            contract: contract.clone(),
            msg: msg.to_string().into_bytes(),
            funds: vec![],
        }
        .to_any()
        .unwrap()];

        let account = account_info(self.url.to_string(), sender.to_string())
            .await
            .map_err(|e| anyhow!("error querying account info: {}", e))?;
        let amount = Coin {
            amount: 11000u128,
            denom: "untrn".parse().expect("hardcoded denom"),
        };
        let tx_bytes = tx_bytes(
            &self.sk,
            amount,
            gas,
            tm_pubkey,
            msgs,
            account.sequence,
            account.account_number,
            chain_id,
        )
        .map_err(|e| anyhow!("failed to create msg/tx: {}", e))?;

        let response = send_tx(self.url.to_string(), tx_bytes)
            .await
            .map_err(|e| anyhow!("failed to send tx: {}", e))?;
        println!("{response:?}");
        Ok(response
            .tx_response
            .map(|tx_response| tx_response.txhash)
            .unwrap_or_default())
    }

    fn deploy<M: ToString>(
        &self,
        _chain_id: &TmChainId,
        _sender: &str,
        _wasm_path: M,
    ) -> Result<String, Self::Error> {
        unimplemented!()
    }

    fn init<M: ToString>(
        &self,
        _chain_id: &TmChainId,
        _sender: &str,
        _code_id: u64,
        _init_msg: M,
        _label: &str,
    ) -> Result<String, Self::Error> {
        unimplemented!()
    }

    fn trusted_height_hash(&self) -> Result<(u64, String), Self::Error> {
        unimplemented!()
    }
}

pub async fn account_info(
    node: impl ToString,
    address: impl ToString,
) -> Result<BaseAccount, Box<dyn Error>> {
    let mut client = AuthQueryClient::connect(node.to_string()).await?;
    let request = tonic::Request::new(QueryAccountRequest {
        address: address.to_string(),
    });
    let response = client.account(request).await?;
    let response = RawBaseAccount::decode(response.into_inner().account.unwrap().value.as_slice())?;
    let account = BaseAccount::try_from(response)?;
    Ok(account)
}

#[allow(clippy::too_many_arguments)]
pub fn tx_bytes(
    secret: &SigningKey,
    amount: Coin,
    gas: u64,
    tm_pubkey: PublicKey,
    msgs: Vec<Any>,
    sequence_number: u64,
    account_number: u64,
    chain_id: &TmChainId,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let tx_body = tx::Body::new(msgs, "", 0u16);
    let signer_info = SignerInfo::single_direct(Some(tm_pubkey), sequence_number);
    let auth_info = signer_info.auth_info(Fee::from_amount_and_gas(amount, gas));
    let sign_doc = SignDoc::new(&tx_body, &auth_info, chain_id, account_number)?;
    let tx_signed = sign_doc.sign(secret)?;
    Ok(tx_signed.to_bytes()?)
}

pub async fn send_tx(
    node: impl ToString,
    tx_bytes: Vec<u8>,
) -> Result<BroadcastTxResponse, Box<dyn Error>> {
    let mut client = ServiceClient::connect(node.to_string()).await?;
    let request = tonic::Request::new(BroadcastTxRequest {
        tx_bytes,
        mode: BroadcastMode::Sync.into(),
    });
    let tx_response = client.broadcast_tx(request).await?;
    Ok(tx_response.into_inner())
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;
    use transfers_contract::msg::{execute::Request, QueryMsg::GetRequests};

    use crate::{grpc::GrpcClient, CwClient};

    #[tokio::test]
    #[ignore]
    async fn test_query() -> Result<(), Box<dyn Error>> {
        let sk = hex::decode("ffc4d3c9119e9e8263de08c0f6e2368ac5c2dacecfeb393f6813da7d178873d2")
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap();
        let url = "https://grpc-falcron.pion-1.ntrn.tech:80".parse().unwrap();
        let contract = "neutron15ruzx9wvrupt9cffzsp6868uad2svhfym2nsgxm2skpeqr3qrd4q4uwk83"
            .parse()
            .unwrap();

        let cw_client = GrpcClient::new(sk, url);
        let resp: Vec<Request> = cw_client
            .query_smart(&contract, json!(GetRequests {}))
            .await?;
        println!("{resp:?}");

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute() -> Result<(), Box<dyn Error>> {
        let sk = hex::decode("ffc4d3c9119e9e8263de08c0f6e2368ac5c2dacecfeb393f6813da7d178873d2")
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap();
        let url = "https://grpc-falcron.pion-1.ntrn.tech:80".parse().unwrap();
        let contract = "neutron15ruzx9wvrupt9cffzsp6868uad2svhfym2nsgxm2skpeqr3qrd4q4uwk83"
            .parse()
            .unwrap();
        let chain_id = "pion-1".parse().unwrap();

        let cw_client = GrpcClient::new(sk, url);
        let tx_hash = cw_client
            .tx_execute(
                &contract,
                &chain_id,
                2000000,
                "/* unused since we're getting the account from the sk */",
                json!([]),
                "11000untrn",
            )
            .await?;
        println!("{}", tx_hash);

        Ok(())
    }
}
