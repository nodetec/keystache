import { create } from "zustand";
import { createJSONStorage, devtools, persist } from "zustand/middleware";

export interface State {
  pubkey: string;
  setPubkey: (pubkey: string) => void;
}

// TODO: persist state to localstorage
const useStore = create<State>()(
  devtools(
    persist(
      (set) => ({
        pubkey: "",
        setPubkey: (pubkey: string) => set({ pubkey }),
      }),
      {
        name: "pubkey-storage",
        storage: createJSONStorage(() => localStorage),
      },
    ),
  ),
);

export default useStore;
