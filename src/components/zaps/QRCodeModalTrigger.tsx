import { useState } from "react";

import { Button } from "~/components/ui/button";
import QRCodeModal from "~/components/zaps/QRCodeModal";
import { UnsignedNostrEvent } from "~/types";

/** 
 *  This component is just a simple temp trigger to easily open 
 *  QRCodeModal for ease of testing.
/*/

type Props = {
  event: UnsignedNostrEvent;
};
const QRCodeModalTrigger = ({ event }: Props) => {
  const [qrCodeOpen, setQRCodeOpen] = useState(false);

  const handleQRCodeOpen = (_open: boolean) => {
    setQRCodeOpen(_open);
  };

  return (
    <>
      <Button
        onClick={() => {
          setQRCodeOpen(true);
        }}
      >
        Open QR Code
      </Button>
      <QRCodeModal
        open={qrCodeOpen}
        handleOpen={handleQRCodeOpen}
        event={event}
      />
    </>
  );
};

export default QRCodeModalTrigger;
