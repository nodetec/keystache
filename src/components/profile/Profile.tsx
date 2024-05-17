import { Filter } from "nostr-tools";
import { RelayUrl, profileContent, useSubscribe } from "react-nostr";
import { Card, CardContent, CardFooter, CardHeader } from "~/components/ui/card";
import { Github, Globe, Zap } from "lucide-react";

type Props = {
  filter: Filter;
  pubkey: string;
};

const eventKey = "profile";
const subRelays: RelayUrl[] = ["wss://nos.lol"];
const BOT_AVATAR_ENDPOINT =
  "https://api.dicebear.com/7.x/bottts-neutral/svg?seed=";

export default function Profile({ filter, pubkey }: Props) {
  const { events } = useSubscribe({
    eventKey,
    filter: filter,
    relays: subRelays,
  });

  return (
    <div className="py-4">
      <Card>
        <CardHeader>
          <div className="flex items-center gap-x-4">
            <img
              src={
                profileContent(events[0]).picture ??
                BOT_AVATAR_ENDPOINT + pubkey
              }
              alt=""
              className="aspect-square w-12 rounded-md border border-border dark:border-border"
            />

            <div className="flex flex-col gap-y-1">
              <span className="text-3xl">{profileContent(events[0]).name}</span>
              <span className="flex items-center gap-x-1">
                {profileContent(events[0]).nip05}
              </span>
            </div>
          </div>
        </CardHeader>
        <CardContent>{profileContent(events[0]).about}</CardContent>
        <CardFooter>
          <div className="flex flex-col gap-y-2">
            {profileContent(events[0]).website && (
              <span className="flex items-center text-sm font-light text-muted-foreground">
                <Globe className="mr-1 h-4 w-4" />
                {profileContent(events[0]).website}
              </span>
            )}

            {profileContent(events[0]).lud16 && (
              <span className="flex items-center text-sm font-light text-muted-foreground">
                <Zap className="mr-1 h-4 w-4" />
                {profileContent(events[0]).lud16}
              </span>
            )}

            {profileContent(events[0]).github && (
              <span className="flex items-center text-sm font-light text-muted-foreground">
                <Github className="mr-1 h-4 w-4" />
                {profileContent(events[0]).github}
              </span>
            )}
          </div>
        </CardFooter>
      </Card>
    </div>
  );
}

