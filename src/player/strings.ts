const english = {
  offlineTitle: "Telai is not running",
  offlineHint: "Open Telai on this computer. This page reconnects by itself.",
  idleTitle: "Waiting for a window",
  idleHint: "Pick a window in Telai to begin.",
  browserTitle: "Use Chrome or Edge",
  browserHint: "This browser does not share tab audio.",
  unlock: "Click to turn on the sound",
  documentTitle: (title: string) => `${title} - Telai`,
};

const portuguese: typeof english = {
  offlineTitle: "Telai não está aberto",
  offlineHint: "Abra o Telai neste computador. Esta página reconecta sozinha.",
  idleTitle: "Aguardando uma janela",
  idleHint: "Escolha uma janela no Telai para começar.",
  browserTitle: "Use o Chrome ou o Edge",
  browserHint: "Este navegador não compartilha o som da aba.",
  unlock: "Clique para ativar o som",
  documentTitle: (title: string) => `${title} - Telai`,
};

export const strings = navigator.language.toLowerCase().startsWith("pt") ? portuguese : english;
