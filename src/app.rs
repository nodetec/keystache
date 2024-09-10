use std::{collections::BTreeMap, sync::Arc};

use fedimint_core::config::FederationId;
use iced::{
    futures::{SinkExt, StreamExt},
    widget::{column, container, row, scrollable},
    Element, Length, Task,
};
use nip_55::nip_46::{Nip46OverNip55ServerStream, Nip46RequestApproval};
use nostr_sdk::PublicKey;

use crate::{
    fedimint::{FederationView, Wallet},
    routes::{self, Route, RouteName},
    ui_components::sidebar,
    ConnectedState,
};

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(RouteName),
    NavigateHomeAndSetConnectedState(ConnectedState),

    UnlockPage(routes::unlock::Message),
    NostrKeypairsPage(routes::nostr_keypairs::Message),
    NostrRelaysPage(routes::nostr_relays::Message),
    BitcoinWalletPage(routes::bitcoin_wallet::Message),
    SettingsPage(routes::settings::Message),

    DbDeleteAllData,

    UpdateFederationViews {
        views: BTreeMap<FederationId, FederationView>,
    },

    CopyStringToClipboard(String),

    IncomingNip46Request(
        Arc<(
            Vec<nostr_sdk::nips::nip46::Request>,
            PublicKey,
            iced::futures::channel::oneshot::Sender<Nip46RequestApproval>,
        )>,
    ),
    ApproveFirstIncomingNip46Request,
    RejectFirstIncomingNip46Request,
}

pub struct App {
    pub page: Route,
}

impl Default for App {
    fn default() -> Self {
        Self {
            page: Route::new_locked(),
        }
    }
}

impl App {
    pub fn update(&mut self, event: Message) -> Task<Message> {
        self.page.update(event)
    }

    pub fn view(&self) -> Element<Message> {
        let Self { page, .. } = self;

        let mut content: Element<Message> = Element::new(scrollable(
            container(column![page.view()].spacing(20).padding(20)).center_x(Length::Fill),
        ));

        if page.to_name() != RouteName::Unlock {
            content = Element::new(row![sidebar(self), content]);
        };

        container(content).center_y(Length::Fill).into()
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let Some(connected_state) = self.page.get_connected_state() else {
            return iced::Subscription::none();
        };

        let db_clone = connected_state.db.clone();

        let wallet = connected_state.wallet.clone();

        let wallet_sub = iced::subscription::channel(
            std::any::TypeId::of::<Wallet>(),
            100,
            |mut output| async move {
                loop {
                    let mut wallet_update_stream = wallet.get_update_stream();

                    while let Some(views) = wallet_update_stream.next().await {
                        output
                            .send(Message::UpdateFederationViews { views })
                            .await
                            .unwrap();
                    }
                }
            },
        );

        let nip46_sub = iced::subscription::channel(
            std::any::TypeId::of::<Nip46OverNip55ServerStream>(),
            100,
            |mut output| async move {
                loop {
                    let mut stream = Nip46OverNip55ServerStream::start(
                        "/tmp/nip55-kind24133.sock",
                        db_clone.clone(),
                    )
                    .unwrap();

                    while let Some((request_list, public_key, response_sender)) =
                        stream.next().await
                    {
                        output
                            .send(Message::IncomingNip46Request(Arc::new((
                                request_list,
                                public_key,
                                response_sender,
                            ))))
                            .await
                            .unwrap();
                    }
                }
            },
        );

        iced::subscription::Subscription::batch(vec![nip46_sub, wallet_sub])
    }
}
