use crate::client::Client;
use crate::srp::compute_srp_check;
use crate::{tl, Error};
use durov_mtproto::transports::Transport;
use std::io::Write;

impl<T: Transport> Client<T>
where
    T: Send + 'static,
{
    pub async fn interactive_login(&mut self, phone: &str) -> Result<(), Error> {
        let sent_code = match self.send_code(phone).await {
            Ok(sent_code) => sent_code,
            Err(Error::RpcError { code: 303, message }) => {
                let dc_id = message.rsplit_once("_")
                    .ok_or_else(|| Error::InvalidRpcError(message.clone()))?
                    .1
                    .parse()
                    .map_err(|_| Error::InvalidRpcError(message.clone()))?;

                self.switch_dc(dc_id).await?;
                self.send_code(phone).await?
            }
            Err(err) => return Err(err),
        };

        let sent_code = match sent_code {
            tl::enums::auth::SentCode::SentCode(sent_code) => sent_code,
            tl::enums::auth::SentCode::SentCodeSuccess(_) => return Ok(()),
            tl::enums::auth::SentCode::SentCodePaymentRequired(_) => unimplemented!("payment required"),
        };
        if matches!(sent_code.type_, tl::enums::auth::SentCodeType::SentCodeTypeSetUpEmailRequired(_)) {
            unimplemented!("set up email required");
        }

        let code = input("code");

        let authorization = match self.sign_in(phone, sent_code, code).await {
            Ok(authorization) => authorization,
            Err(Error::RpcError { code: 401, message }) if message == "SESSION_PASSWORD_NEEDED" => {
                let pwd = self.call(tl::functions::account::GetPassword {}).await?;
                let password = input("password");
                let check = compute_srp_check(pwd, &password)?;
                self.call(tl::functions::auth::CheckPassword { password: check }).await?
            }
            Err(err) => return Err(err),
        };

        if matches!(authorization, tl::enums::auth::Authorization::AuthorizationSignUpRequired(_)) {
            unimplemented!("sign up required");
        }

        // todo: store user data

        Ok(())
    }

    pub async fn bot_login(&self, token: &str) -> Result<(), Error> {
        self.call(tl::functions::auth::ImportBotAuthorization {
            flags: 0,
            api_id: self.config.api_id,
            api_hash: self.config.api_hash.clone(),
            bot_auth_token: token.to_string(),
        }).await?;

        Ok(())
    }

    async fn send_code(&self, phone: &str) -> Result<tl::enums::auth::SentCode, Error> {
        self.call(tl::functions::auth::SendCode {
            phone_number: phone.to_string(),
            api_id: self.config.api_id,
            api_hash: self.config.api_hash.clone(),
            settings: tl::types::CodeSettings {
                allow_flashcall: false,
                current_number: false,
                allow_app_hash: false,
                allow_missed_call: false,
                allow_firebase: false,
                unknown_number: false,
                logout_tokens: None,
                token: None,
                app_sandbox: None,
            }.into(),
        }).await
    }

    async fn sign_in(&self, phone: &str, sent_code: tl::types::auth::SentCode, code: String)
        -> Result<tl::enums::auth::Authorization, Error>
    {
        self.call(tl::functions::auth::SignIn {
            phone_number: phone.to_string(),
            phone_code_hash: sent_code.phone_code_hash,
            phone_code: Some(code),
            email_verification: None,
        }).await
    }
}

fn input(what: &str) -> String {
    print!("Enter {what}: ");
    std::io::stdout()
        .flush()
        .unwrap();
    std::io::stdin()
        .lines()
        .next()
        .unwrap()
        .unwrap()
}
