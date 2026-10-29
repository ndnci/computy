// Reviewed maintenance consent text; no online translation at runtime.
import { readFileSync, writeFileSync } from 'node:fs';
const keys = [
  'Installation et données',
  'Désinstaller Computy',
  'Supprimer toutes les données',
  'L’historique, les préférences et les imports en attente seront supprimés. Les exports personnels sont conservés.',
  'Le système demandera votre autorisation. Les données sont conservées.',
  'Version portable : supprimez le fichier ou le dossier de l’application.',
  'Cette action est irréversible.',
];
const rows = {
  en: 'Installation and data|Uninstall Computy|Erase all data|History, preferences and pending imports will be deleted. Personal exports are kept.|The system will ask for your permission. Data is kept.|Portable version: delete the application file or folder.|This action cannot be undone.',
  es: 'Instalación y datos|Desinstalar Computy|Borrar todos los datos|Se eliminarán el historial, las preferencias y las importaciones pendientes. Se conservan las exportaciones personales.|El sistema solicitará tu autorización. Se conservan los datos.|Versión portátil: elimina el archivo o la carpeta de la aplicación.|Esta acción no se puede deshacer.',
  'pt-BR':
    'Instalação e dados|Desinstalar Computy|Apagar todos os dados|O histórico, as preferências e as importações pendentes serão excluídos. As exportações pessoais serão mantidas.|O sistema pedirá sua autorização. Os dados serão mantidos.|Versão portátil: exclua o arquivo ou a pasta do aplicativo.|Esta ação não pode ser desfeita.',
  de: 'Installation und Daten|Computy deinstallieren|Alle Daten löschen|Verlauf, Einstellungen und ausstehende Importe werden gelöscht. Persönliche Exporte bleiben erhalten.|Das System fragt nach Ihrer Erlaubnis. Die Daten bleiben erhalten.|Portable Version: Löschen Sie die Anwendungsdatei oder den Ordner.|Diese Aktion kann nicht rückgängig gemacht werden.',
  it: 'Installazione e dati|Disinstalla Computy|Elimina tutti i dati|Cronologia, preferenze e importazioni in attesa saranno eliminate. Le esportazioni personali saranno conservate.|Il sistema chiederà la tua autorizzazione. I dati saranno conservati.|Versione portatile: elimina il file o la cartella dell’applicazione.|Questa azione è irreversibile.',
  nl: 'Installatie en gegevens|Computy verwijderen|Alle gegevens wissen|Geschiedenis, voorkeuren en wachtende imports worden verwijderd. Persoonlijke exports blijven bewaard.|Het systeem vraagt om toestemming. Gegevens blijven bewaard.|Draagbare versie: verwijder het toepassingsbestand of de map.|Deze actie kan niet ongedaan worden gemaakt.',
  pl: 'Instalacja i dane|Odinstaluj Computy|Usuń wszystkie dane|Historia, preferencje i oczekujące importy zostaną usunięte. Własne eksporty zostaną zachowane.|System poprosi o zgodę. Dane zostaną zachowane.|Wersja przenośna: usuń plik lub folder aplikacji.|Tej operacji nie można cofnąć.',
  ru: 'Установка и данные|Удалить Computy|Удалить все данные|История, настройки и ожидающие импорты будут удалены. Личные экспорты сохранятся.|Система запросит разрешение. Данные сохранятся.|Переносная версия: удалите файл или папку приложения.|Это действие нельзя отменить.',
  uk: 'Встановлення та дані|Видалити Computy|Видалити всі дані|Історію, налаштування та імпорти в очікуванні буде видалено. Особисті експорти залишаться.|Система запитає ваш дозвіл. Дані залишаться.|Портативна версія: видаліть файл або папку програми.|Цю дію неможливо скасувати.',
  tr: 'Kurulum ve veriler|Computy uygulamasını kaldır|Tüm verileri sil|Geçmiş, tercihler ve bekleyen içe aktarmalar silinir. Kişisel dışa aktarmalar korunur.|Sistem izninizi isteyecektir. Veriler korunur.|Taşınabilir sürüm: uygulama dosyasını veya klasörünü silin.|Bu işlem geri alınamaz.',
  ar: 'التثبيت والبيانات|إلغاء تثبيت Computy|حذف جميع البيانات|سيتم حذف السجل والتفضيلات وعمليات الاستيراد المعلقة. ستبقى الملفات المصدرة الشخصية.|سيطلب النظام إذنك. ستبقى البيانات محفوظة.|النسخة المحمولة: احذف ملف التطبيق أو مجلده.|لا يمكن التراجع عن هذا الإجراء.',
  hi: 'इंस्टॉलेशन और डेटा|Computy अनइंस्टॉल करें|सारा डेटा मिटाएँ|इतिहास, प्राथमिकताएँ और लंबित आयात मिटा दिए जाएँगे। व्यक्तिगत निर्यात सुरक्षित रहेंगे।|सिस्टम आपकी अनुमति माँगेगा। डेटा सुरक्षित रहेगा।|पोर्टेबल संस्करण: एप्लिकेशन की फ़ाइल या फ़ोल्डर मिटाएँ।|इस कार्रवाई को वापस नहीं लिया जा सकता।',
  bn: 'ইনস্টলেশন ও ডেটা|Computy আনইনস্টল করুন|সমস্ত ডেটা মুছুন|ইতিহাস, পছন্দ এবং অপেক্ষমাণ আমদানি মুছে যাবে। ব্যক্তিগত রপ্তানি রাখা হবে।|সিস্টেম আপনার অনুমতি চাইবে। ডেটা রাখা হবে।|পোর্টেবল সংস্করণ: অ্যাপের ফাইল বা ফোল্ডার মুছুন।|এই কাজটি পূর্বাবস্থায় ফেরানো যাবে না।',
  ur: 'تنصیب اور ڈیٹا|Computy اَن انسٹال کریں|تمام ڈیٹا مٹائیں|تاریخ، ترجیحات اور زیر التوا درآمدات مٹا دی جائیں گی۔ ذاتی برآمدات محفوظ رہیں گی۔|سسٹم آپ کی اجازت مانگے گا۔ ڈیٹا محفوظ رہے گا۔|پورٹیبل ورژن: ایپ کی فائل یا فولڈر حذف کریں۔|اس عمل کو واپس نہیں لیا جا سکتا۔',
  id: 'Instalasi dan data|Hapus instalasi Computy|Hapus semua data|Riwayat, preferensi, dan impor tertunda akan dihapus. Ekspor pribadi tetap disimpan.|Sistem akan meminta izin Anda. Data tetap disimpan.|Versi portabel: hapus berkas atau folder aplikasi.|Tindakan ini tidak dapat dibatalkan.',
  vi: 'Cài đặt và dữ liệu|Gỡ cài đặt Computy|Xóa tất cả dữ liệu|Lịch sử, tùy chọn và mục nhập đang chờ sẽ bị xóa. Các tệp xuất cá nhân được giữ lại.|Hệ thống sẽ yêu cầu bạn cho phép. Dữ liệu được giữ lại.|Bản di động: xóa tệp hoặc thư mục ứng dụng.|Không thể hoàn tác hành động này.',
  th: 'การติดตั้งและข้อมูล|ถอนการติดตั้ง Computy|ลบข้อมูลทั้งหมด|ประวัติ การตั้งค่า และการนำเข้าที่รอดำเนินการจะถูกลบ ไฟล์ส่งออกส่วนตัวจะยังอยู่|ระบบจะขออนุญาตจากคุณ ข้อมูลจะยังคงอยู่|รุ่นพกพา: ลบไฟล์หรือโฟลเดอร์ของแอป|การดำเนินการนี้ไม่สามารถยกเลิกได้',
  ko: '설치 및 데이터|Computy 제거|모든 데이터 삭제|기록, 환경설정 및 대기 중인 가져오기가 삭제됩니다. 개인 내보내기 파일은 유지됩니다.|시스템이 권한을 요청합니다. 데이터는 유지됩니다.|휴대용 버전: 앱 파일 또는 폴더를 삭제하세요.|이 작업은 되돌릴 수 없습니다.',
  ja: 'インストールとデータ|Computy をアンインストール|すべてのデータを削除|履歴、設定、保留中のインポートを削除します。個人のエクスポートは保持されます。|システムが許可を求めます。データは保持されます。|ポータブル版：アプリのファイルまたはフォルダーを削除してください。|この操作は取り消せません。',
  'zh-CN':
    '安装与数据|卸载 Computy|删除所有数据|历史记录、偏好设置和待处理导入将被删除。个人导出文件将保留。|系统将请求您的授权。数据将保留。|便携版：删除应用程序文件或文件夹。|此操作无法撤销。',
  'zh-TW':
    '安裝與資料|解除安裝 Computy|刪除所有資料|歷史記錄、偏好設定及待處理匯入將被刪除。個人匯出檔案會保留。|系統將要求您的授權。資料會保留。|可攜版：刪除應用程式檔案或資料夾。|此操作無法復原。',
  fa: 'نصب و داده‌ها|حذف نصب Computy|پاک کردن همه داده‌ها|تاریخچه، تنظیمات و واردسازی‌های در انتظار حذف می‌شوند. خروجی‌های شخصی حفظ می‌شوند.|سیستم اجازه شما را درخواست می‌کند. داده‌ها حفظ می‌شوند.|نسخه قابل حمل: فایل یا پوشه برنامه را حذف کنید.|این کار قابل بازگشت نیست.',
  he: 'התקנה ונתונים|הסרת Computy|מחיקת כל הנתונים|ההיסטוריה, ההעדפות והייבוא הממתין יימחקו. קובצי ייצוא אישיים יישמרו.|המערכת תבקש את אישורך. הנתונים יישמרו.|גרסה ניידת: יש למחוק את קובץ היישום או את התיקייה.|לא ניתן לבטל פעולה זו.',
  sw: 'Usakinishaji na data|Ondoa Computy|Futa data zote|Historia, mapendeleo na uagizaji unaosubiri vitafutwa. Faili binafsi zilizohamishwa zitahifadhiwa.|Mfumo utaomba ruhusa yako. Data itahifadhiwa.|Toleo linalobebeka: futa faili au folda ya programu.|Hatua hii haiwezi kutenduliwa.',
};
const source = JSON.parse(readFileSync('src/locales/source.json'));
const catalogs = JSON.parse(readFileSync('src/locales/catalogs.json'));
const overrides = JSON.parse(readFileSync('src/locales/overrides.json'));
for (const key of keys) source[key] = key;
for (const [lang, row] of Object.entries(rows)) {
  const values = row.split('|');
  if (values.length !== keys.length) throw Error(lang);
  const additions = Object.fromEntries(keys.map((key, index) => [key, values[index]]));
  Object.assign(catalogs[lang], additions);
  Object.assign(overrides[lang], additions);
}
for (const [file, value] of Object.entries({ source, catalogs, overrides }))
  writeFileSync(`src/locales/${file}.json`, JSON.stringify(value, null, 2) + '\n');
