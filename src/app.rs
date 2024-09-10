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
    db::Database,
    fedimint::{FederationView, Wallet},
    routes::{self, bitcoin_wallet, unlock, Loadable, Route, RouteName},
    ui_components::sidebar,
};

#[derive(Debug, Clone)]
pub enum Message {
    Routes(routes::Message),

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
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Routes(routes_msg) => self.page.update(routes_msg),
            Message::DbDeleteAllData => {
                if let Route::Unlock(unlock::Page {
                    db_already_exists, ..
                }) = &mut self.page
                {
                    Database::delete();
                    *db_already_exists = false;
                }

                Task::none()
            }
            Message::UpdateFederationViews { views } => {
                if let Some(connected_state) = self.page.get_connected_state_mut() {
                    connected_state.loadable_federation_views = Loadable::Loaded(views.clone());
                }

                if let Route::BitcoinWallet(bitcoin_wallet) = &mut self.page {
                    bitcoin_wallet.update(bitcoin_wallet::Message::UpdateFederationViews(views));
                }

                Task::none()
            }
            Message::CopyStringToClipboard(text) => {
                // TODO: Display a toast stating whether the copy succeeded or failed.
                let _ = arboard::Clipboard::new().map(|mut clipboard| clipboard.set_text(text));

                Task::none()
            }
            Message::IncomingNip46Request(data) => {
                if let Some(connected_state) = self.page.get_connected_state_mut() {
                    connected_state.in_flight_nip46_requests.push_back(data);
                }

                Task::none()
            }
            Message::ApproveFirstIncomingNip46Request => {
                if let Some(connected_state) = self.page.get_connected_state_mut() {
                    if let Some(req) = connected_state.in_flight_nip46_requests.pop_front() {
                        let req = Arc::try_unwrap(req).unwrap();
                        req.2.send(Nip46RequestApproval::Approve).unwrap();
                    }
                }

                Task::none()
            }
            Message::RejectFirstIncomingNip46Request => {
                if let Some(connected_state) = self.page.get_connected_state_mut() {
                    if let Some(req) = connected_state.in_flight_nip46_requests.pop_front() {
                        let req = Arc::try_unwrap(req).unwrap();
                        req.2.send(Nip46RequestApproval::Reject).unwrap();
                    }
                }

                Task::none()
            }
        }
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
