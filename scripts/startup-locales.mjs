import { readFileSync, writeFileSync } from 'node:fs';
const keys = [
  'Démarrage automatique',
  'Afficher les indicateurs à la connexion',
  'Démarre le moniteur en arrière-plan, sans ouvrir la fenêtre principale.',
  'Le moniteur démarrera à la connexion.',
];
const rows = {
  en: 'Login startup|Show indicators at login|Starts monitoring in the background without opening the main window.|Monitoring will start at login.',
  es: 'Inicio automático|Mostrar indicadores al iniciar sesión|Inicia el monitor en segundo plano sin abrir la ventana principal.|El monitor se iniciará al iniciar sesión.',
  'pt-BR':
    'Inicialização automática|Mostrar indicadores ao entrar|Inicia o monitor em segundo plano sem abrir a janela principal.|O monitor iniciará ao entrar.',
  de: 'Automatischer Start|Anzeigen bei der Anmeldung einblenden|Startet die Überwachung im Hintergrund, ohne das Hauptfenster zu öffnen.|Die Überwachung startet bei der Anmeldung.',
  it: 'Avvio automatico|Mostra gli indicatori all’accesso|Avvia il monitoraggio in background senza aprire la finestra principale.|Il monitoraggio si avvierà all’accesso.',
  nl: 'Automatisch starten|Indicatoren tonen bij aanmelden|Start de monitor op de achtergrond zonder het hoofdvenster te openen.|De monitor start bij het aanmelden.',
  pl: 'Automatyczne uruchamianie|Pokaż wskaźniki po zalogowaniu|Uruchamia monitor w tle bez otwierania głównego okna.|Monitor uruchomi się po zalogowaniu.',
  ru: 'Автозапуск|Показывать индикаторы при входе|Запускает мониторинг в фоне без открытия главного окна.|Мониторинг запустится при входе.',
  uk: 'Автозапуск|Показувати індикатори після входу|Запускає моніторинг у фоні без відкриття головного вікна.|Моніторинг запуститься після входу.',
  tr: 'Otomatik başlatma|Oturum açıldığında göstergeleri göster|Ana pencereyi açmadan izlemeyi arka planda başlatır.|İzleme oturum açıldığında başlayacak.',
  ar: 'التشغيل التلقائي|عرض المؤشرات عند تسجيل الدخول|يبدأ المراقبة في الخلفية دون فتح النافذة الرئيسية.|ستبدأ المراقبة عند تسجيل الدخول.',
  hi: 'स्वचालित शुरुआत|लॉगिन पर संकेतक दिखाएँ|मुख्य विंडो खोले बिना पृष्ठभूमि में निगरानी शुरू करता है।|लॉगिन पर निगरानी शुरू होगी।',
  bn: 'স্বয়ংক্রিয় চালু|লগইনে সূচক দেখান|মূল উইন্ডো না খুলে পটভূমিতে পর্যবেক্ষণ শুরু করে।|লগইনে পর্যবেক্ষণ শুরু হবে।',
  ur: 'خودکار آغاز|لاگ اِن پر اشارے دکھائیں|مرکزی ونڈو کھولے بغیر پس منظر میں نگرانی شروع کرتا ہے۔|لاگ اِن پر نگرانی شروع ہوگی۔',
  id: 'Mulai otomatis|Tampilkan indikator saat masuk|Memulai pemantauan di latar belakang tanpa membuka jendela utama.|Pemantauan akan dimulai saat masuk.',
  vi: 'Khởi động tự động|Hiện chỉ báo khi đăng nhập|Bắt đầu giám sát nền mà không mở cửa sổ chính.|Giám sát sẽ bắt đầu khi đăng nhập.',
  th: 'เริ่มอัตโนมัติ|แสดงตัวบ่งชี้เมื่อเข้าสู่ระบบ|เริ่มตรวจสอบเบื้องหลังโดยไม่เปิดหน้าต่างหลัก|การตรวจสอบจะเริ่มเมื่อเข้าสู่ระบบ',
  ko: '자동 시작|로그인 시 표시기 표시|기본 창을 열지 않고 백그라운드 모니터링을 시작합니다.|로그인 시 모니터링이 시작됩니다.',
  ja: '自動起動|ログイン時にインジケーターを表示|メインウィンドウを開かずにバックグラウンドで監視を開始します。|ログイン時に監視が開始されます。',
  'zh-CN': '自动启动|登录时显示指示器|在后台启动监控，不打开主窗口。|登录时将启动监控。',
  'zh-TW': '自動啟動|登入時顯示指示器|在背景啟動監控，不開啟主視窗。|登入時將啟動監控。',
  fa: 'شروع خودکار|نمایش نشانگرها هنگام ورود|نظارت را در پس‌زمینه بدون باز کردن پنجره اصلی آغاز می‌کند.|نظارت هنگام ورود آغاز خواهد شد.',
  he: 'הפעלה אוטומטית|הצגת מחוונים בכניסה|מפעיל ניטור ברקע בלי לפתוח את החלון הראשי.|הניטור יתחיל בכניסה.',
  sw: 'Kuanzisha kiotomatiki|Onyesha viashiria wakati wa kuingia|Huanzisha ufuatiliaji chinichini bila kufungua dirisha kuu.|Ufuatiliaji utaanza wakati wa kuingia.',
};
const source = JSON.parse(readFileSync('src/locales/source.json'));
const catalogs = JSON.parse(readFileSync('src/locales/catalogs.json'));
const overrides = JSON.parse(readFileSync('src/locales/overrides.json'));
for (const key of keys) source[key] = key;
for (const [lang, row] of Object.entries(rows)) {
  const values = row.split('|');
  if (values.length !== keys.length) throw Error(lang);
  const entries = Object.fromEntries(keys.map((key, i) => [key, values[i]]));
  Object.assign(catalogs[lang], entries);
  Object.assign(overrides[lang], entries);
}
for (const [name, value] of Object.entries({ source, catalogs, overrides }))
  writeFileSync(`src/locales/${name}.json`, JSON.stringify(value, null, 2) + '\n');
