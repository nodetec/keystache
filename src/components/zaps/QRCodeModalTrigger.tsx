import { useState } from "react";

import { Button } from "~/components/ui/button";
import QRCodeModal from "~/components/zaps/QRCodeModal";

type Props = {
  invoice: string;
};
const QRCodeModalTrigger = ({ invoice }: Props) => {
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
        invoice={invoice}
      />
    </>
  );
};

export default QRCodeModalTrigger;
