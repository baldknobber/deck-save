import { createContext, useContext } from "react";

interface SetupContextType {
  setNeedsSetup: (v: boolean) => void;
}

export const SetupContext = createContext<SetupContextType>({
  setNeedsSetup: () => {},
});

export function useSetup() {
  return useContext(SetupContext);
}
