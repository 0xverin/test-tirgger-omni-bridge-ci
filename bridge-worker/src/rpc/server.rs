use crate::keystore::KeyStore as KeyStoreT;
use crate::rpc::methods::*;
use crate::shielding_key::ShieldingKey;
use jsonrpsee::server::tracing::info;
use jsonrpsee::server::Server;
use jsonrpsee::RpcModule;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::runtime::Handle;

pub struct RpcContext<KeyStore: KeyStoreT> {
    pub import_keystore_signer: [u8; 33],
    pub keystore: Arc<RwLock<KeyStore>>,
    pub shielding_key: Arc<ShieldingKey>,
}

// pass server context here
pub async fn start_server<KeyStore: KeyStoreT>(
    address: &str,
    handle: Handle,
    import_keystore_signer: [u8; 33],
    keystore: Arc<RwLock<KeyStore>>,
    shielding_key: Arc<ShieldingKey>,
) -> SocketAddr {
    let server = Server::builder()
        .custom_tokio_runtime(handle)
        .build(address.parse::<SocketAddr>().unwrap())
        .await
        .unwrap();

    let context = RpcContext { import_keystore_signer, keystore, shielding_key };
    let mut module = RpcModule::new(context);

    register_get_shielding_key(&mut module);
    register_import_relayer_key(&mut module);

    let addr = server.local_addr().unwrap();
    info!("Server listening on {}", addr);
    let handle = server.start(module);
    tokio::spawn(handle.stopped());

    addr
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::alice_signer;
    use crate::keystore::LocalKeystore;
    use crate::rpc::error_code::*;
    use crate::shielding_key::ShieldingKey;
    use jsonrpsee::types::{Response, ResponsePayload};
    use jsonrpsee_core::JsonRawValue;
    use rand::rngs::OsRng;
    use rsa::Oaep;
    use rsa::RsaPrivateKey;
    use sha2::Sha256;
    use sp_core::{keccak_256, Pair};
    use std::fs;
    use std::path::PathBuf;

    const SR25519_SEED: &str = "e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a";

    struct GlobalContext;

    impl GlobalContext {
        fn setup() -> Arc<ShieldingKey> {
            let key: RsaPrivateKey = serde_json::from_str(r#"{"pubkey_components":{"n":[2902428985,3030084423,71537824,330754855,3088797530,1333155746,3859933877,998043133,4055167788,3389012306,414103315,3855441301,2334833976,3369046619,3538498605,2746527914,505459908,845990733,1298711580,190369169,1328267733,1696672341,1495772496,1511448680,4145071828,140608394,2297500098,2984248725,1912305352,1483157512,3692838439,2977072868,676024582,1379582626,667314503,3651057317,2069712232,4158124598,4059421562,1359740247,2022094059,2750077769,2174451857,1688571952,2058140893,511747482,3614418731,2787560495,4148688666,76513831,4186445569,3537487226,1286425607,1012062486,3900813829,1860007907,2348321810,3120665752,447535708,2179351338,3487052681,451136099,2305104172,2992699609],"e":[65537]},"d":[1851887737,788315507,4013827738,4104826729,3054521037,3276705785,2951892502,3039679951,804862396,1332155767,2271144796,1641949574,231874588,3561673887,1619692480,2505224679,433961249,4235940283,2484132214,3219441937,1510111112,291682213,3952000133,170983406,953118686,3426738443,269314623,2219322476,1788928358,1460965968,328690546,1375179643,672540328,457232945,569667616,14044575,1608034390,117000477,3878825344,3977344438,2944131697,1920048131,4111418776,473833721,793268564,354634409,188675642,2521946821,2355115849,2943664041,1331219024,1632940625,374491971,3030516214,1886173359,3886763897,2782445697,1076627759,1130238476,1546950846,1690429799,132713663,1515254134,2110694132],"primes":[[3402250683,3294881972,4169609523,3070676552,1390461968,3135233523,1387965320,2921458458,113033400,861030721,1895694789,164820657,2056610536,3025931362,2880155889,2648713933,202802821,1895399657,221009069,464126633,2350011559,313845561,4204037651,1170988860,3674462967,1098213417,1448933802,170906521,967995194,3055210519,2974236951,3400220960],[4185504667,958320937,2880429174,11763834,1858530037,2599253162,4079864083,90305400,3470432005,2746586703,21726232,473851216,207680176,46359070,3665997773,1779833430,3773343740,1973821220,2298822812,1534145284,774424605,2950971609,1067680631,786009521,3551602666,1688612497,3482722671,429126127,723782424,3574166391,1318821239,3780209315]]}"#).unwrap();
            Arc::new(ShieldingKey::init_with(key))
        }
    }

    #[test]
    pub fn print_sig() {
        let key = sp_core::ecdsa::Pair::from_string("//Alice", None).unwrap();
        let w = ImportRelayerKeyPayload { id: "rococo".to_string(), key: hex::decode("3bac64ca36d1a64c0c70ff4759f47246253d4fab94e1316e98fb038b7a55bb95fd741f38bbd779ed6b8c0264789f9fac398aba8071c68aa17ee23251eb1e12dd90f92ea9942ee9018075a9c317353b51ceb545caa210d8deb47de356912def894bbb2c77159054fe04f55c661cee218abe7b51e8c37d122a51fd88645664e167b3827a324c37a9d557cc6200f78941a6e225735a441c17d2a1e48c494c32b7317f08b2ff461ef5e8caa9e92960b79a559c0a7b3eff954528bad87f2ffc92fe2ca57bc43c59b48a88f7b4f2f5dd4bcacaec1565967e9eb8131f8db5b69606920560d441de41402e6e0526733ac6f4a1f970b103f62739cf8c4c038376e8ff4100").unwrap() };
        let data = serde_json::to_vec(&w).unwrap();
        let sig = key.sign_prehashed(&keccak_256(&data)).0;
        println!("payload is: {}, sig is {}", serde_json::to_string(&w).unwrap(), hex::encode(sig));
    }

    #[tokio::test]
    pub async fn unthorized_request_should_fail() {
        let shielding_key = GlobalContext::setup();
        let data_dir: PathBuf = "unthorized_request_should_fail".into();
        fs::create_dir_all(&data_dir).unwrap();
        let keystore = Arc::new(RwLock::new(LocalKeystore::open(data_dir.clone()).unwrap()));

        let address = start_server("127.0.0.1:2003", Handle::current(), alice_signer(), keystore, shielding_key).await;

        let client = reqwest::Client::new();

        let body = r#"
        {
            "jsonrpc": "2.0",
            "method": "hm_importRelayerKey",
            "params": {
                "payload": {"id":"rococo", "key":"3bac64ca36d1a64c0c70ff4759f47246253d4fab94e1316e98fb038b7a55bb95fd741f38bbd779ed6b8c0264789f9fac398aba8071c68aa17ee23251eb1e12dd90f92ea9942ee9018075a9c317353b51ceb545caa210d8deb47de356912def894bbb2c77159054fe04f55c661cee218abe7b51e8c37d122a51fd88645664e167b3827a324c37a9d557cc6200f78941a6e225735a441c17d2a1e48c494c32b7317f08b2ff461ef5e8caa9e92960b79a559c0a7b3eff954528bad87f2ffc92fe2ca57bc43c59b48a88f7b4f2f5dd4bcacaec1565967e9eb8131f8db5b69606920560d441de41402e6e0526733ac6f4a1f970b103f62739cf8c4c038376e8ff4100"},
                "signature": "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
            },
            "id": "5"
        }
        "#;

        let response = client
            .post(format!("http://{}", address.to_string()))
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await
            .unwrap();

        let response_bytes = &response.bytes().await.unwrap();
        let json_rpc_response =
            Response::try_from(serde_json::from_slice::<Response<&JsonRawValue>>(response_bytes).unwrap()).unwrap();

        assert!(
            matches!(json_rpc_response.payload, ResponsePayload::Error(e) if e.code() == UNAUTHORIZED_REQUEST_CODE )
        );
        fs::remove_dir_all(data_dir).unwrap();
    }

    #[tokio::test]
    pub async fn get_shielding_key_works() {
        let shielding_key = GlobalContext::setup();
        let data_dir: PathBuf = "get_shielding_key_works".into();
        fs::create_dir_all(&data_dir).unwrap();
        let keystore = Arc::new(RwLock::new(LocalKeystore::open(data_dir.clone()).unwrap()));

        let address = start_server("127.0.0.1:2004", Handle::current(), alice_signer(), keystore, shielding_key).await;

        let client = reqwest::Client::new();

        let body = r#"
        {
            "jsonrpc": "2.0",
            "method": "hm_getShieldingKey",
            "params": {},
            "id": "5"
        }
        "#;

        let response = client
            .post(format!("http://{}", address.to_string()))
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await
            .unwrap();

        let response_bytes = &response.bytes().await.unwrap();
        let json_rpc_response =
            Response::try_from(serde_json::from_slice::<Response<&JsonRawValue>>(response_bytes).unwrap()).unwrap();

        assert!(matches!(
          json_rpc_response.payload,
          ResponsePayload::Success(b) if b.get() == r#"{"e":"010001","n":"398dffac476b9bb4a094430427ebb6135a4f1bb8a257764fb5ea11e6fded7c3b2cf3b4f1523900ca13b7ae18955dcde538bd2a8b5b92cfc82d34e9d2aab0b4a3c4b4201e4dcb6c321cc4684d91cd580bd5c12b4f552a216550ad275968e0165ad4c610f78a836108c211f1889505e0b1c876fb7108306758273e1cdce48672b106514b28a2c23a524769c627a5b69ed9684d5d7b36f2d7f77adbf5f157fd0b51ebb4867849dbeaa391809b813090a564ddbcac7a9aa5801e2ba76fd72fcc26a61af747f727828f04011788f97ac5d9d2074cad4c16d9523c05b281e8e377dd6e128cf88b989401ba5cdaac1a2a43e6818933d8cf63cae31a2c196589d9f860b2"}"#
        ));
        fs::remove_dir_all(data_dir).unwrap();
    }

    #[tokio::test]
    pub async fn import_relayer_key_works() {
        let shielding_key = GlobalContext::setup();
        let data_dir: PathBuf = "import_relayer_key_works".into();
        fs::create_dir_all(&data_dir).unwrap();
        let keystore = Arc::new(RwLock::new(LocalKeystore::open(data_dir.clone()).unwrap()));

        let _shielded_key = shielding_key
            .public_key()
            .encrypt(&mut OsRng, Oaep::new::<Sha256>(), hex::decode(SR25519_SEED).unwrap().as_slice())
            .unwrap();

        let address = start_server("127.0.0.1:2005", Handle::current(), alice_signer(), keystore, shielding_key).await;

        let client = reqwest::Client::new();

        let body = r#"
        {
            "jsonrpc": "2.0",
            "method": "hm_importRelayerKey",
            "params": {
                "payload": {"id":"rococo", "key":"3bac64ca36d1a64c0c70ff4759f47246253d4fab94e1316e98fb038b7a55bb95fd741f38bbd779ed6b8c0264789f9fac398aba8071c68aa17ee23251eb1e12dd90f92ea9942ee9018075a9c317353b51ceb545caa210d8deb47de356912def894bbb2c77159054fe04f55c661cee218abe7b51e8c37d122a51fd88645664e167b3827a324c37a9d557cc6200f78941a6e225735a441c17d2a1e48c494c32b7317f08b2ff461ef5e8caa9e92960b79a559c0a7b3eff954528bad87f2ffc92fe2ca57bc43c59b48a88f7b4f2f5dd4bcacaec1565967e9eb8131f8db5b69606920560d441de41402e6e0526733ac6f4a1f970b103f62739cf8c4c038376e8ff4100"},
                "signature": "6f3b1b29361cfddbc84a6ae6d192e983a20c73e6f6aad3942c234d9f99e218fd129796424864c56b1263cc9246c18cfa21965045a2f5c9f8c1527dc309bfbbbd01"
            },
            "id": "5"
        }
        "#;

        let response = client
            .post(format!("http://{}", address.to_string()))
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await
            .unwrap();

        let response_bytes = &response.bytes().await.unwrap();

        let json_rpc_response =
            Response::try_from(serde_json::from_slice::<Response<&JsonRawValue>>(response_bytes).unwrap()).unwrap();
        assert!(matches!(json_rpc_response.payload, ResponsePayload::Success(_)));

        let path: PathBuf = data_dir.join("rococo.bin");
        assert!(path.is_file());
        let read_key = fs::read(path).unwrap();
        assert_eq!(read_key, hex::decode(SR25519_SEED).unwrap());
        fs::remove_dir_all(data_dir).unwrap();
    }
}
