use std::collections::BTreeSet;
use std::{collections::BTreeMap, str::FromStr};

use fedimint_core::config::FederationId;
use fedimint_core::{
    config::{ClientConfig, META_FEDERATION_NAME_KEY},
    invite_code::InviteCode,
};
use iced::widget::{container::Style, Container};
use iced::{
    futures::{stream::FuturesUnordered, StreamExt},
    widget::{text_input, Column, Text},
    Border, Element, Shadow, Task,
};
use iced::{Length, Theme};
use nostr_sdk::PublicKey;

use crate::util::lighten;
use crate::{
    app,
    routes::{self, container, ConnectedState, Loadable},
    ui_components::{icon_button, PaletteColor, SvgIcon, Toast, ToastStatus},
    util::truncate_text,
};

#[derive(Debug, Clone)]
pub enum Message {
    InviteCodeInputChanged(String),

    LoadedFederationConfigFromInviteCode {
        // The invite code that was used to load the federation config.
        invite_code: InviteCode,
        // The loaded federation config from the federation that the invite code belongs to.
        config: ClientConfig,
    },
    FailedToLoadFederationConfigFromInviteCode {
        // The invite code that was used to attempt to load the federation config.
        invite_code: InviteCode,
    },

    JoinFederation(InviteCode),
    JoinedFederation(InviteCode),

    LoadNip87Federations,
    LoadedNip87Federations(BTreeMap<FederationId, (BTreeSet<PublicKey>, BTreeSet<InviteCode>)>),
}

pub struct Page {
    connected_state: ConnectedState,
    invite_code_input: String,
    parsed_invite_code_state_or: Option<ParsedInviteCodeState>,
    // TODO: Simplify this type and remove the clippy warning.
    #[allow(clippy::type_complexity)]
    nip_87_data_or:
        Option<Loadable<BTreeMap<FederationId, (BTreeSet<PublicKey>, Vec<ParsedInviteCodeState>)>>>,
}

struct ParsedInviteCodeState {
    invite_code: InviteCode,
    loadable_federation_config: Loadable<ClientConfig>,
}

impl Page {
    pub fn new(connected_state: &ConnectedState) -> Self {
        Self {
            connected_state: connected_state.clone(),
            invite_code_input: String::new(),
            parsed_invite_code_state_or: None,
            nip_87_data_or: None,
        }
    }

    // TODO: Remove this clippy allow.
    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, msg: Message) -> Task<app::Message> {
        match msg {
            Message::InviteCodeInputChanged(new_invite_code_input) => {
                self.invite_code_input = new_invite_code_input;

                let Ok(invite_code) = InviteCode::from_str(&self.invite_code_input) else {
                    self.parsed_invite_code_state_or = None;

                    return Task::none();
                };

                self.parsed_invite_code_state_or = Some(ParsedInviteCodeState {
                    invite_code: invite_code.clone(),
                    loadable_federation_config: Loadable::Loading,
                });

                Task::future(async {
                    match fedimint_api_client::download_from_invite_code(&invite_code).await {
                        Ok(config) => app::Message::Routes(routes::Message::BitcoinWalletPage(
                            super::Message::Add(Message::LoadedFederationConfigFromInviteCode {
                                invite_code,
                                config,
                            }),
                        )),
                        Err(_err) => app::Message::Routes(routes::Message::BitcoinWalletPage(
                            super::Message::Add(
                                Message::FailedToLoadFederationConfigFromInviteCode { invite_code },
                            ),
                        )),
                    }
                })
            }
            Message::LoadedFederationConfigFromInviteCode {
                invite_code,
                config,
            } => {
                if let Some(ParsedInviteCodeState {
                    invite_code: parsed_invite_code,
                    loadable_federation_config,
                }) = &mut self.parsed_invite_code_state_or
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    if &invite_code == parsed_invite_code {
                        *loadable_federation_config = Loadable::Loaded(config.clone());
                    }
                }

                self.handle_client_config_outcome_for_invite_code(
                    &invite_code,
                    &Loadable::Loaded(config),
                );

                Task::none()
            }
            Message::FailedToLoadFederationConfigFromInviteCode { invite_code } => {
                if let Some(ParsedInviteCodeState {
                    invite_code: parsed_invite_code,
                    loadable_federation_config,
                }) = &mut self.parsed_invite_code_state_or
                {
                    // If the invite code has changed since the request was made, ignore the response.
                    // Also only update the state if the user attempted to load the config.
                    if &invite_code == parsed_invite_code
                        && matches!(loadable_federation_config, Loadable::Loading)
                    {
                        *loadable_federation_config = Loadable::Failed;
                    }
                }

                self.handle_client_config_outcome_for_invite_code(&invite_code, &Loadable::Failed);

                // TODO: Show toast instead of returning an empty task.
                Task::none()
            }
            Message::JoinFederation(invite_code) => {
                let wallet = self.connected_state.wallet.clone();

                Task::stream(async_stream::stream! {
                    match wallet.join_federation(invite_code.clone()).await {
                        Ok(()) => {
                            yield app::Message::AddToast(Toast {
                                title: "Joined federation".to_string(),
                                body: "You have successfully joined the federation.".to_string(),
                                status: ToastStatus::Good,
                            });

                            yield app::Message::Routes(routes::Message::BitcoinWalletPage(super::Message::Add(
                                Message::JoinedFederation(invite_code)
                            )));
                        }
                        Err(err) => {
                            yield app::Message::AddToast(Toast {
                                title: "Failed to join federation".to_string(),
                                body: format!("Failed to join the federation: {err}"),
                                status: ToastStatus::Bad,
                            });
                        }
                    }
                })
            }
            Message::JoinedFederation(invite_code) => {
                // If the invite code matches the one that was just joined, navigate back to the `List` page.
                if let Some(invite_code_state) = &self.parsed_invite_code_state_or {
                    if invite_code_state.invite_code == invite_code {
                        return Task::done(app::Message::Routes(routes::Message::Navigate(
                            routes::RouteName::BitcoinWallet(super::SubrouteName::List),
                        )));
                    }
                }

                Task::none()
            }
            Message::LoadNip87Federations => {
                self.nip_87_data_or = Some(Loadable::Loading);

                let nostr_module = self.connected_state.nostr_module.clone();

                Task::future(async move {
                    match nostr_module.find_federations().await {
                        Ok(federations) => {
                            app::Message::Routes(routes::Message::BitcoinWalletPage(
                                super::Message::Add(Message::LoadedNip87Federations(federations)),
                            ))
                        }
                        Err(_err) => app::Message::AddToast(Toast {
                            title: "Failed to discover federations".to_string(),
                            body: "Nostr NIP-87 federation discovery failed.".to_string(),
                            status: ToastStatus::Bad,
                        }),
                    }
                })
            }
            Message::LoadedNip87Federations(nip_87_data) => {
                // Only set the state to loaded if the user requested the data.
                // This prevents the data from being displayed if the user
                // navigates away from the page and back before the data is loaded.
                if matches!(self.nip_87_data_or, Some(Loadable::Loading)) {
                    self.nip_87_data_or = Some(Loadable::Loaded(
                        nip_87_data
                            .clone()
                            .into_iter()
                            .map(|(federation_id, (pubkeys, invite_codes))| {
                                (
                                    federation_id,
                                    (
                                        pubkeys,
                                        invite_codes
                                            .into_iter()
                                            .map(|invite_code| ParsedInviteCodeState {
                                                invite_code,
                                                loadable_federation_config: Loadable::Loading,
                                            })
                                            .collect(),
                                    ),
                                )
                            })
                            .collect(),
                    ));
                }

                Task::stream(async_stream::stream! {
                    let mut futures = FuturesUnordered::new();

                    for (_, (_, invite_codes)) in nip_87_data {
                        if let Some(invite_code) = invite_codes.first().cloned() {
                            futures.push(async move {
                                match fedimint_api_client::download_from_invite_code(&invite_code).await {
                                    Ok(config) => {
                                        app::Message::Routes(routes::Message::BitcoinWalletPage(
                                            super::Message::Add(Message::LoadedFederationConfigFromInviteCode {
                                                invite_code: invite_code.clone(),
                                                config,
                                            }),
                                        ))
                                    }
                                    // TODO: Include error in message and display it in the UI.
                                    Err(_err) => {
                                        app::Message::Routes(routes::Message::BitcoinWalletPage(
                                            super::Message::Add(Message::FailedToLoadFederationConfigFromInviteCode {
                                                invite_code: invite_code.clone(),
                                            }),
                                        ))
                                    }
                                }
                            });
                        }
                    }

                    while let Some(result) = futures.next().await {
                        yield result;
                    }
                })
            }
        }
    }

    // TODO: Remove this clippy exception.
    #[allow(clippy::too_many_lines)]
    pub fn view<'a>(&self) -> Column<'a, app::Message> {
        let mut container = container("Join Federation")
            .push(
                text_input("Federation Invite Code", &self.invite_code_input)
                    .on_input(|input| {
                        app::Message::Routes(routes::Message::BitcoinWalletPage(
                            super::Message::Add(Message::InviteCodeInputChanged(input)),
                        ))
                    })
                    .padding(10)
                    .size(30),
            )
            .push(
                icon_button("Join Federation", SvgIcon::Groups, PaletteColor::Primary)
                    .on_press_maybe(self.parsed_invite_code_state_or.as_ref().map(
                        |parsed_federation_invite_code_state| {
                            app::Message::Routes(routes::Message::BitcoinWalletPage(
                                super::Message::Add(Message::JoinFederation(
                                    parsed_federation_invite_code_state.invite_code.clone(),
                                )),
                            ))
                        },
                    )),
            );

        if let Some(parsed_federation_invite_code_state) = &self.parsed_invite_code_state_or {
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

            match &parsed_federation_invite_code_state.loadable_federation_config {
                Loadable::Loading => {
                    container = container.push(Text::new("Loading..."));
                }
                Loadable::Loaded(client_config) => {
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
                Loadable::Failed => {
                    container = container.push(Text::new("Failed to load client config"));
                }
            }
        }

        let nip_87_view: Element<app::Message> = match &self.nip_87_data_or {
            None => icon_button(
                "Find Federations",
                SvgIcon::Search,
                PaletteColor::Background,
            )
            .on_press(app::Message::Routes(routes::Message::BitcoinWalletPage(
                super::Message::Add(Message::LoadNip87Federations),
            )))
            .into(),
            Some(Loadable::Loading) => Text::new("Loading...").into(),
            Some(Loadable::Loaded(federation_data)) => {
                let mut column = Column::new().spacing(10);

                let mut federation_data_sorted_by_recommendations: Vec<_> = federation_data
                    .iter()
                    .map(|(federation_id, (pubkeys, invite_codes))| {
                        (federation_id, pubkeys, invite_codes)
                    })
                    .collect();

                federation_data_sorted_by_recommendations
                    .sort_by_key(|(_, pubkeys, _)| pubkeys.len());
                federation_data_sorted_by_recommendations.reverse();

                // Filter out federations that we're already connected to.
                if let Loadable::Loaded(wallet_view) = &self.connected_state.loadable_wallet_view {
                    let connected_federation_ids =
                        wallet_view.federations.keys().collect::<BTreeSet<_>>();

                    federation_data_sorted_by_recommendations.retain(|(federation_id, _, _)| {
                        !connected_federation_ids.contains(federation_id)
                    });
                }

                for (federation_id, pubkeys, invite_codes) in
                    federation_data_sorted_by_recommendations
                {
                    let mut sub_column = Column::new()
                        .push(Text::new(format!("Federation ID: {federation_id}")))
                        .push(Text::new(format!("{} recommendations", pubkeys.len())));

                    let mut loading_invite_codes: Vec<&ParsedInviteCodeState> = Vec::new();
                    let mut loaded_invite_codes: Vec<&ParsedInviteCodeState> = Vec::new();
                    let mut errored_invite_codes: Vec<&ParsedInviteCodeState> = Vec::new();
                    for invite_code in invite_codes {
                        match &invite_code.loadable_federation_config {
                            Loadable::Loading => {
                                loading_invite_codes.push(invite_code);
                            }
                            Loadable::Loaded(_) => {
                                loaded_invite_codes.push(invite_code);
                            }
                            Loadable::Failed => {
                                errored_invite_codes.push(invite_code);
                            }
                        }
                    }

                    let mut most_progressed_invite_code_or = None;
                    // The order of priority is errored, loading, loaded.
                    // This is important because we don't want to consider a
                    // federation as errored if one of its invite codes is loading, and
                    // we don't want to consider a federation as loading if one of its
                    // invite codes has successfully loaded.
                    if !errored_invite_codes.is_empty() {
                        most_progressed_invite_code_or = Some(errored_invite_codes[0]);
                    } else if !loading_invite_codes.is_empty() {
                        most_progressed_invite_code_or = Some(loading_invite_codes[0]);
                    } else if !loaded_invite_codes.is_empty() {
                        most_progressed_invite_code_or = Some(loaded_invite_codes[0]);
                    }

                    if let Some(most_progressed_invite_code) = most_progressed_invite_code_or {
                        match &most_progressed_invite_code.loadable_federation_config {
                            Loadable::Loading => {
                                sub_column = sub_column.push(Text::new("Loading client config..."));
                            }
                            Loadable::Loaded(client_config) => {
                                sub_column = sub_column
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
                                    sub_column = sub_column.push(Text::new(format!(
                                        "{} ({})",
                                        peer_url.name, peer_url.url
                                    )));
                                }

                                sub_column = sub_column.push(
                                    icon_button(
                                        "Join Federation",
                                        SvgIcon::Groups,
                                        PaletteColor::Primary,
                                    )
                                    .on_press(
                                        app::Message::Routes(routes::Message::BitcoinWalletPage(
                                            super::Message::Add(Message::JoinFederation(
                                                most_progressed_invite_code.invite_code.clone(),
                                            )),
                                        )),
                                    ),
                                );
                            }
                            Loadable::Failed => {
                                sub_column =
                                    sub_column.push(Text::new("Failed to load client config"));
                            }
                        }
                    }

                    column = column.push(
                        Container::new(sub_column)
                            .padding(10)
                            .width(Length::Fill)
                            .style(|theme: &Theme| -> Style {
                                Style {
                                    text_color: None,
                                    background: Some(
                                        lighten(theme.palette().background, 0.05).into(),
                                    ),
                                    border: Border {
                                        color: iced::Color::WHITE,
                                        width: 0.0,
                                        radius: (8.0).into(),
                                    },
                                    shadow: Shadow::default(),
                                }
                            }),
                    );
                }

                column.into()
            }
            Some(Loadable::Failed) => Text::new("Failed to load NIP-87 data").into(),
        };

        container = container.push(nip_87_view);

        container = container.push(
            icon_button("Back", SvgIcon::ArrowBack, PaletteColor::Background).on_press(
                app::Message::Routes(routes::Message::Navigate(super::RouteName::BitcoinWallet(
                    super::SubrouteName::List,
                ))),
            ),
        );

        container
    }

    /// Handle the outcome of a client config request from a given invite code.
    fn handle_client_config_outcome_for_invite_code(
        &mut self,
        invite_code: &InviteCode,
        loadable_client_config: &Loadable<ClientConfig>,
    ) {
        if let Some(Loadable::Loaded(nip_87_data)) = &mut self.nip_87_data_or {
            for (_, nip_87_invite_codes) in nip_87_data.values_mut() {
                for nip_87_invite_code in nip_87_invite_codes {
                    if &nip_87_invite_code.invite_code == invite_code
                        && nip_87_invite_code.loadable_federation_config == Loadable::Loading
                    {
                        nip_87_invite_code.loadable_federation_config =
                            loadable_client_config.clone();
                    }
                }
            }
        }
    }
}
