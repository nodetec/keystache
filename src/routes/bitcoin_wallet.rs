use fedimint_core::{
    config::{ClientConfig, META_FEDERATION_NAME_KEY},
    invite_code::InviteCode,
};
use iced::widget::{text_input, Column, Text};

use crate::{
    ui_components::{icon_button, PaletteColor, SvgIcon},
    util::{format_amount_sats, truncate_text},
    ConnectedState, KeystacheMessage,
};

use super::container;

#[derive(Clone)]
pub struct Page {
    pub connected_state: ConnectedState,
    pub federation_invite_code: String,
    pub parsed_federation_invite_code_state_or: Option<ParsedFederationInviteCodeState>,
}

#[derive(Clone)]
pub struct ParsedFederationInviteCodeState {
    pub invite_code: InviteCode,
    pub maybe_loading_federation_config: MaybeLoadingFederationConfig,
}

#[derive(Clone)]
pub enum MaybeLoadingFederationConfig {
    Loading,
    Loaded(ClientConfig),
    Failed,
}

impl Page {
    // TODO: Remove this clippy allow.
    #[allow(clippy::unused_self)]
    pub fn view<'a>(&self) -> Column<'a, KeystacheMessage> {
        let mut container = container("Wallet")
            .push(Text::new(format!("Balance: {}", format_amount_sats(0))))
            .push(
                text_input("Federation Invite Code", &self.federation_invite_code)
                    .on_input(KeystacheMessage::JoinFederationInviteCodeInputChanged)
                    .padding(10)
                    .size(30),
            )
            .push(
                icon_button("Join Federation", SvgIcon::Groups, PaletteColor::Primary)
                    .on_press_maybe(self.parsed_federation_invite_code_state_or.as_ref().map(
                        |parsed_federation_invite_code_state| {
                            KeystacheMessage::JoinFedimintFederation(
                                parsed_federation_invite_code_state.invite_code.clone(),
                            )
                        },
                    )),
            );

        if let Some(parsed_federation_invite_code_state) =
            &self.parsed_federation_invite_code_state_or
        {
            container = container
                .push(Text::new("Federation ID").size(25))
                .push(Text::new(truncate_text(
                    &parsed_federation_invite_code_state
                        .invite_code
                        .federation_id()
                        .to_string(),
                    21,
                    true,
                )));

            match &parsed_federation_invite_code_state.maybe_loading_federation_config {
                MaybeLoadingFederationConfig::Loading => {
                    container = container.push(Text::new("Loading..."));
                }
                MaybeLoadingFederationConfig::Loaded(client_config) => {
                    container = container
                        .push(Text::new("Federation Name").size(25))
                        .push(Text::new(
                            client_config
                                .meta::<String>(META_FEDERATION_NAME_KEY)
                                .ok()
                                .flatten()
                                .unwrap_or_default(),
                        ))
                        .push(Text::new("Modules").size(25))
                        .push(Text::new(
                            client_config
                                .modules
                                .values()
                                .map(|module| module.kind().to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                        ))
                        .push(Text::new("Guardians").size(25));
                    for peer_url in client_config.global.api_endpoints.values() {
                        container = container
                            .push(Text::new(format!("{} ({})", peer_url.name, peer_url.url)));
                    }
                }
                MaybeLoadingFederationConfig::Failed => {
                    container = container.push(Text::new("Failed to load client config"));
                }
            }
        }

        container
    }
}
