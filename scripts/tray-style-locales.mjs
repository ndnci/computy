import { readFileSync, writeFileSync } from 'node:fs';
const keys = [
  'Style de la barre système',
  'Icône + pourcentage',
  'Anneau',
  'Icône remplie',
  'Par défaut',
  'Bleu',
  'Jaune',
  'Rouge',
];
const rows = {
  en: 'System tray style|Icon + percentage|Ring|Filled icon|Default|Blue|Yellow|Red',
  es: 'Estilo de la bandeja del sistema|Icono + porcentaje|Anillo|Icono relleno|Predeterminado|Azul|Amarillo|Rojo',
  'pt-BR':
    'Estilo da bandeja do sistema|Ícone + porcentagem|Anel|Ícone preenchido|Padrão|Azul|Amarelo|Vermelho',
  de: 'Stil der Statusleiste|Symbol + Prozent|Ring|Gefülltes Symbol|Standard|Blau|Gelb|Rot',
  it: 'Stile della barra di sistema|Icona + percentuale|Anello|Icona riempita|Predefinito|Blu|Giallo|Rosso',
  nl: 'Stijl van het systeemvak|Pictogram + percentage|Ring|Gevuld pictogram|Standaard|Blauw|Geel|Rood',
  pl: 'Styl zasobnika systemowego|Ikona + procent|Pierścień|Wypełniona ikona|Domyślny|Niebieski|Żółty|Czerwony',
  ru: 'Стиль системного трея|Значок + процент|Кольцо|Заполняемый значок|По умолчанию|Синий|Жёлтый|Красный',
  uk: 'Стиль системного трея|Піктограма + відсоток|Кільце|Заповнювана піктограма|За замовчуванням|Синій|Жовтий|Червоний',
  tr: 'Sistem tepsisi stili|Simge + yüzde|Halka|Dolgu simgesi|Varsayılan|Mavi|Sarı|Kırmızı',
  ar: 'نمط شريط النظام|أيقونة + نسبة مئوية|حلقة|أيقونة ممتلئة|افتراضي|أزرق|أصفر|أحمر',
  hi: 'सिस्टम ट्रे की शैली|आइकन + प्रतिशत|रिंग|भरने वाला आइकन|डिफ़ॉल्ट|नीला|पीला|लाल',
  bn: 'সিস্টেম ট্রের ধরন|আইকন + শতাংশ|রিং|ভরা আইকন|ডিফল্ট|নীল|হলুদ|লাল',
  ur: 'سسٹم ٹرے کا انداز|آئیکن + فیصد|حلقہ|بھرتا ہوا آئیکن|ڈیفالٹ|نیلا|پیلا|سرخ',
  id: 'Gaya baki sistem|Ikon + persentase|Cincin|Ikon terisi|Bawaan|Biru|Kuning|Merah',
  vi: 'Kiểu khay hệ thống|Biểu tượng + phần trăm|Vòng tròn|Biểu tượng tô đầy|Mặc định|Xanh dương|Vàng|Đỏ',
  th: 'รูปแบบถาดระบบ|ไอคอน + เปอร์เซ็นต์|วงแหวน|ไอคอนเติมสี|ค่าเริ่มต้น|น้ำเงิน|เหลือง|แดง',
  ko: '시스템 트레이 스타일|아이콘 + 백분율|링|채워지는 아이콘|기본값|파랑|노랑|빨강',
  ja: 'システムトレイの表示|アイコン＋使用率|リング|塗りつぶしアイコン|既定|青|黄|赤',
  'zh-CN': '系统托盘样式|图标＋百分比|圆环|填充图标|默认|蓝色|黄色|红色',
  'zh-TW': '系統匣樣式|圖示＋百分比|圓環|填滿圖示|預設|藍色|黃色|紅色',
  fa: 'سبک سینی سیستم|نماد + درصد|حلقه|نماد پُرشونده|پیش‌فرض|آبی|زرد|قرمز',
  he: 'סגנון מגש המערכת|סמל + אחוזים|טבעת|סמל מתמלא|ברירת מחדל|כחול|צהוב|אדום',
  sw: 'Mtindo wa eneo la arifa|Aikoni + asilimia|Pete|Aikoni inayojazwa|Chaguo-msingi|Bluu|Njano|Nyekundu',
};

const catalogs = JSON.parse(readFileSync('src/locales/catalogs.json', 'utf8'));
const overrides = JSON.parse(readFileSync('src/locales/overrides.json', 'utf8'));
for (const [lang, row] of Object.entries(rows)) {
  const values = row.split('|');
  if (values.length !== keys.length) throw Error(lang);
  const additions = Object.fromEntries(keys.map((key, i) => [key, values[i]]));
  additions['En direct dans votre barre système.'] = {
    en: 'Live in your system tray.',
    es: 'En directo en la bandeja del sistema.',
    'pt-BR': 'Ao vivo na bandeja do sistema.',
    de: 'Live in Ihrer Statusleiste.',
    it: 'In tempo reale nella barra di sistema.',
    nl: 'Live in het systeemvak.',
    pl: 'Na żywo w zasobniku systemowym.',
    ru: 'В реальном времени в системном трее.',
    uk: 'У реальному часі в системному треї.',
    tr: 'Sistem tepsisinde canlı ölçümler.',
    ar: 'قياسات مباشرة في شريط النظام.',
    hi: 'सिस्टम ट्रे में लाइव जानकारी।',
    bn: 'সিস্টেম ট্রেতে সরাসরি তথ্য।',
    ur: 'سسٹم ٹرے میں براہ راست معلومات۔',
    id: 'Langsung di baki sistem Anda.',
    vi: 'Trực tiếp trên khay hệ thống.',
    th: 'ข้อมูลสดในถาดระบบ',
    ko: '시스템 트레이에서 실시간으로 확인하세요.',
    ja: 'システムトレイでリアルタイムに確認。',
    'zh-CN': '在系统托盘中实时查看。',
    'zh-TW': '在系統匣中即時查看。',
    fa: 'نمایش زنده در سینی سیستم.',
    he: 'נתונים חיים במגש המערכת.',
    sw: 'Taarifa za moja kwa moja katika eneo la arifa.',
  }[lang];

  for (const key of ['65–89 %', '< 65 %', '≥ 90 %', 'SSD / HDD']) additions[key] = key;
  additions['Commande non autorisée.'] = {
    en: 'Command not allowed.',
    es: 'Comando no autorizado.',
    'pt-BR': 'Comando não autorizado.',
    de: 'Befehl nicht erlaubt.',
    it: 'Comando non autorizzato.',
    nl: 'Opdracht niet toegestaan.',
    pl: 'Niedozwolone polecenie.',
    ru: 'Команда не разрешена.',
    uk: 'Команду не дозволено.',
    tr: 'Komuta izin verilmiyor.',
    ar: 'الأمر غير مسموح.',
    hi: 'कमांड की अनुमति नहीं है।',
    bn: 'এই কমান্ডের অনুমতি নেই।',
    ur: 'اس کمانڈ کی اجازت نہیں ہے۔',
    id: 'Perintah tidak diizinkan.',
    vi: 'Lệnh không được phép.',
    th: 'ไม่อนุญาตให้ใช้คำสั่งนี้',
    ko: '허용되지 않는 명령입니다.',
    ja: 'このコマンドは許可されていません。',
    'zh-CN': '不允许此命令。',
    'zh-TW': '不允許此命令。',
    fa: 'این فرمان مجاز نیست.',
    he: 'הפקודה אינה מורשית.',
    sw: 'Amri hairuhusiwi.',
  }[lang];
  Object.assign(catalogs[lang], additions);
  Object.assign(overrides[lang], additions);
}
writeFileSync('src/locales/catalogs.json', JSON.stringify(catalogs, null, 2) + '\n');
writeFileSync('src/locales/overrides.json', JSON.stringify(overrides, null, 2) + '\n');
