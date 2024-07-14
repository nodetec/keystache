export interface UnsignedNostrEvent {
  id: string;
  pubkey: string;
  created_at: number;
  kind: number;
  tags: string[][];
  content: string;
}

export type PayInvoiceRequestHandler = (
  invoice: string,
) => Promise<boolean> | boolean;
