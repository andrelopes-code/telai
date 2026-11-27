const english = {
  offlineTitle: "Telai is not running",
  offlineHint: "Open Telai on this computer. This page reconnects by itself.",
  idleTitle: "No window is being streamed",
  idleHint: "Pick a window in Telai.",
  audioTitle: "Click to turn on the sound",
  audioHint: "The browser plays audio only after a click on this page.",
  browserHint: "Use Chrome or Edge: this browser does not share tab audio.",
  documentTitle: (title: string) => `${title} - Telai`,
};

const portuguese: typeof english = {
  offlineTitle: "O Telai não está aberto",
  offlineHint: "Abra o Telai neste computador. Esta página conecta sozinha.",
  idleTitle: "Nenhuma janela em transmissão",
  idleHint: "Escolha uma janela no Telai.",
  audioTitle: "Clique para ativar o som",
  audioHint: "O navegador só toca áudio depois de um clique nesta página.",
  browserHint: "Use o Chrome ou o Edge: este navegador não compartilha o som da aba.",
  documentTitle: (title: string) => `${title} - Telai`,
};

export const strings = navigator.language.toLowerCase().startsWith("pt") ? portuguese : english;
