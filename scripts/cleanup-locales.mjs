// Reviewed navigation, Boolean operators and destructive-action labels.
import { readFileSync, writeFileSync } from 'node:fs';
const keys = [
  'Nettoyage',
  'Applications',
  'Ventilateurs',
  'Toutes les conditions (ET)',
  'Une condition au moins (OU)',
  'Exclure si',
  'Confirmer la suppression',
  'Confirmer la désinstallation',
  'Uniquement les fichiers dans node_modules',
];
const rows = {
  en: 'Cleanup|Applications|Fans|All conditions (AND)|Any condition (OR)|Exclude if|Confirm deletion|Confirm uninstall|Only files inside node_modules',
  es: 'Limpieza|Aplicaciones|Ventiladores|Todas las condiciones (Y)|Al menos una condición (O)|Excluir si|Confirmar eliminación|Confirmar desinstalación|Solo archivos dentro de node_modules',
  'pt-BR':
    'Limpeza|Aplicativos|Ventoinhas|Todas as condições (E)|Pelo menos uma condição (OU)|Excluir se|Confirmar exclusão|Confirmar desinstalação|Somente arquivos em node_modules',
  de: 'Bereinigung|Anwendungen|Lüfter|Alle Bedingungen (UND)|Mindestens eine Bedingung (ODER)|Ausschließen, wenn|Löschen bestätigen|Deinstallation bestätigen|Nur Dateien in node_modules',
  it: 'Pulizia|Applicazioni|Ventole|Tutte le condizioni (E)|Almeno una condizione (O)|Escludi se|Conferma eliminazione|Conferma disinstallazione|Solo file in node_modules',
  nl: 'Opschonen|Toepassingen|Ventilatoren|Alle voorwaarden (EN)|Minstens één voorwaarde (OF)|Uitsluiten als|Verwijdering bevestigen|De-installatie bevestigen|Alleen bestanden in node_modules',
  pl: 'Czyszczenie|Aplikacje|Wentylatory|Wszystkie warunki (I)|Co najmniej jeden warunek (LUB)|Wyklucz, jeśli|Potwierdź usunięcie|Potwierdź odinstalowanie|Tylko pliki w node_modules',
  ru: 'Очистка|Приложения|Вентиляторы|Все условия (И)|Хотя бы одно условие (ИЛИ)|Исключить, если|Подтвердить удаление|Подтвердить удаление приложения|Только файлы внутри node_modules',
  uk: 'Очищення|Програми|Вентилятори|Усі умови (І)|Принаймні одна умова (АБО)|Виключити, якщо|Підтвердити видалення|Підтвердити видалення програми|Лише файли в node_modules',
  tr: 'Temizleme|Uygulamalar|Fanlar|Tüm koşullar (VE)|En az bir koşul (VEYA)|Şu durumda hariç tut|Silmeyi onayla|Kaldırmayı onayla|Yalnızca node_modules içindeki dosyalar',
  ar: 'التنظيف|التطبيقات|المراوح|جميع الشروط (و)|شرط واحد على الأقل (أو)|استبعاد إذا|تأكيد الحذف|تأكيد إلغاء التثبيت|الملفات داخل node_modules فقط',
  hi: 'सफ़ाई|ऐप्लिकेशन|पंखे|सभी शर्तें (और)|कम से कम एक शर्त (या)|इस स्थिति में बाहर रखें|हटाने की पुष्टि करें|अनइंस्टॉल की पुष्टि करें|केवल node_modules के अंदर की फ़ाइलें',
  bn: 'পরিষ্কার|অ্যাপ্লিকেশন|ফ্যান|সব শর্ত (এবং)|অন্তত একটি শর্ত (অথবা)|এই ক্ষেত্রে বাদ দিন|মুছে ফেলা নিশ্চিত করুন|আনইনস্টল নিশ্চিত করুন|শুধু node_modules-এর ভেতরের ফাইল',
  ur: 'صفائی|ایپلی کیشنز|پنکھے|تمام شرائط (اور)|کم از کم ایک شرط (یا)|اس صورت میں خارج کریں|حذف کرنے کی تصدیق کریں|اَن انسٹال کرنے کی تصدیق کریں|صرف node_modules کے اندر کی فائلیں',
  id: 'Pembersihan|Aplikasi|Kipas|Semua kondisi (DAN)|Minimal satu kondisi (ATAU)|Kecualikan jika|Konfirmasi penghapusan|Konfirmasi pencopotan|Hanya berkas di dalam node_modules',
  vi: 'Dọn dẹp|Ứng dụng|Quạt|Tất cả điều kiện (VÀ)|Ít nhất một điều kiện (HOẶC)|Loại trừ nếu|Xác nhận xóa|Xác nhận gỡ cài đặt|Chỉ các tệp trong node_modules',
  th: 'การล้างข้อมูล|แอปพลิเคชัน|พัดลม|ทุกเงื่อนไข (และ)|อย่างน้อยหนึ่งเงื่อนไข (หรือ)|ยกเว้นเมื่อ|ยืนยันการลบ|ยืนยันการถอนการติดตั้ง|เฉพาะไฟล์ใน node_modules',
  ko: '정리|애플리케이션|팬|모든 조건 (그리고)|하나 이상의 조건 (또는)|다음 경우 제외|삭제 확인|제거 확인|node_modules 안의 파일만',
  ja: 'クリーンアップ|アプリケーション|ファン|すべての条件（AND）|いずれかの条件（OR）|次の場合は除外|削除を確認|アンインストールを確認|node_modules 内のファイルのみ',
  'zh-CN':
    '清理|应用程序|风扇|全部条件（与）|至少一个条件（或）|满足以下条件时排除|确认删除|确认卸载|仅 node_modules 内的文件',
  'zh-TW':
    '清理|應用程式|風扇|所有條件（且）|至少一個條件（或）|符合以下條件時排除|確認刪除|確認解除安裝|僅 node_modules 內的檔案',
  fa: 'پاک‌سازی|برنامه‌ها|فن‌ها|همهٔ شرط‌ها (و)|حداقل یک شرط (یا)|مستثنا شود اگر|تأیید حذف|تأیید حذف برنامه|فقط فایل‌های داخل node_modules',
  he: 'ניקוי|יישומים|מאווררים|כל התנאים (וגם)|לפחות תנאי אחד (או)|לא לכלול אם|אישור מחיקה|אישור הסרת התקנה|רק קבצים בתוך node_modules',
  sw: 'Usafishaji|Programu|Feni|Masharti yote (NA)|Angalau sharti moja (AU)|Ondoa ikiwa|Thibitisha kufuta|Thibitisha kusanidua|Faili zilizo ndani ya node_modules pekee',
};
const volumeLabels = {
  en: 'Include unused Docker volumes',
  es: 'Incluir volúmenes Docker sin usar',
  'pt-BR': 'Incluir volumes Docker não utilizados',
  de: 'Ungenutzte Docker-Volumes einbeziehen',
  it: 'Includi volumi Docker inutilizzati',
  nl: 'Ongebruikte Docker-volumes meenemen',
  pl: 'Uwzględnij nieużywane woluminy Docker',
  ru: 'Включить неиспользуемые тома Docker',
  uk: 'Додати невикористані томи Docker',
  tr: 'Kullanılmayan Docker birimlerini dahil et',
  ar: 'تضمين وحدات تخزين Docker غير المستخدمة',
  hi: 'अप्रयुक्त Docker वॉल्यूम शामिल करें',
  bn: 'অব্যবহৃত Docker ভলিউম অন্তর্ভুক্ত করুন',
  ur: 'غیر استعمال شدہ Docker والیوم شامل کریں',
  id: 'Sertakan volume Docker yang tidak digunakan',
  vi: 'Bao gồm các ổ Docker không sử dụng',
  th: 'รวมโวลุ่ม Docker ที่ไม่ได้ใช้',
  ko: '사용하지 않는 Docker 볼륨 포함',
  ja: '未使用の Docker ボリュームを含める',
  'zh-CN': '包含未使用的 Docker 卷',
  'zh-TW': '包含未使用的 Docker 磁碟區',
  fa: 'شامل حجم‌های استفاده‌نشدهٔ Docker',
  he: 'לכלול אמצעי אחסון של Docker שאינם בשימוש',
  sw: 'Jumuisha volume za Docker zisizotumika',
};
const catalogs = JSON.parse(readFileSync('src/locales/catalogs.json'));
const overrides = JSON.parse(readFileSync('src/locales/overrides.json'));
for (const [language, row] of Object.entries(rows)) {
  const values = row.split('|');
  if (values.length !== keys.length) throw Error(language);
  const entries = Object.fromEntries(keys.map((k, i) => [k, values[i]]));
  entries['Docker'] = 'Docker';
  entries['Docker ·'] = 'Docker ·';
  entries['Inclure les volumes Docker inutilisés'] = volumeLabels[language];
  Object.assign(catalogs[language], entries);
  Object.assign(overrides[language], entries);
}
for (const [name, value] of Object.entries({ catalogs, overrides }))
  writeFileSync(`src/locales/${name}.json`, JSON.stringify(value, null, 2) + '\n');
