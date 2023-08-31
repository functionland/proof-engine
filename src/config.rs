use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub fula_sugarfunge_api_host: String,
    pub fula_contract_api_host: String,
}

pub fn init() -> Config {
    let panic_message: String = "enviroment variable is not set".to_string();

    Config {
        fula_sugarfunge_api_host: match env::var("FULA_SUGARFUNGE_API_HOST") {
            Ok(var) => var,
            Err(_) => panic!("FULA_SUGARFUNGE_API_HOST {}", panic_message),
        },
        fula_contract_api_host: match env::var("FULA_CONTRACT_API_HOST") {
            Ok(var) => var,
            Err(_) => panic!("FULA_CONTRACT_API_HOST {}", panic_message),
        },
    }
}
