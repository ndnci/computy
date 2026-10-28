import { writeFileSync } from 'node:fs';
const keys =
  'Vue d’ensemble|Diagnostics|Mémoire vive|Stockage|Processeur|Historique|Comparaison|Préférences|À propos de Computy|Langue de l’interface|Système|Clair|Sombre|Suivre le système|Enregistrer les préférences|Lancer le diagnostic|Nouveau diagnostic|Non vérifié|Conforme au test|Conforme|Anomalie détectée|Anomalie|À examiner|Interrompu|Non communiqué|Oui|Non|Annuler|Supprimer le bilan|Exporter en JSON|Rechercher un diagnostic…|Tout voir|Actualiser|Arrêter le diagnostic|Comparer les bilans|Choisir des bilans|Premier regard|L’essentiel|Plus en détail'.split(
    '|',
  );
const rows = {
  en: 'Overview|Diagnostics|Memory|Storage|Processor|History|Comparison|Preferences|About Computy|Interface language|System|Light|Dark|Follow system|Save preferences|Run diagnostic|New diagnostic|Not verified|Test passed|Passed|Issue detected|Issue|Needs review|Interrupted|Not reported|Yes|No|Cancel|Delete report|Export JSON|Search diagnostics…|View all|Refresh|Stop diagnostic|Compare reports|Choose reports|Quick check|Essentials|In depth',
  es: 'Vista general|Diagnósticos|Memoria RAM|Almacenamiento|Procesador|Historial|Comparación|Preferencias|Acerca de Computy|Idioma de la interfaz|Sistema|Claro|Oscuro|Seguir el sistema|Guardar preferencias|Iniciar diagnóstico|Nuevo diagnóstico|Sin verificar|Prueba superada|Correcto|Problema detectado|Problema|Revisar|Interrumpido|No disponible|Sí|No|Cancelar|Eliminar informe|Exportar JSON|Buscar un diagnóstico…|Ver todo|Actualizar|Detener diagnóstico|Comparar informes|Elegir informes|Comprobación rápida|Esencial|En profundidad',
  'pt-BR':
    'Visão geral|Diagnósticos|Memória RAM|Armazenamento|Processador|Histórico|Comparação|Preferências|Sobre o Computy|Idioma da interface|Sistema|Claro|Escuro|Seguir o sistema|Salvar preferências|Iniciar diagnóstico|Novo diagnóstico|Não verificado|Teste aprovado|Aprovado|Problema detectado|Problema|Requer análise|Interrompido|Não informado|Sim|Não|Cancelar|Excluir relatório|Exportar JSON|Buscar diagnóstico…|Ver tudo|Atualizar|Parar diagnóstico|Comparar relatórios|Escolher relatórios|Verificação rápida|Essencial|Em detalhes',
  de: 'Übersicht|Diagnosen|Arbeitsspeicher|Speicherlaufwerke|Prozessor|Verlauf|Vergleich|Einstellungen|Über Computy|Sprache der Oberfläche|System|Hell|Dunkel|Systemeinstellung verwenden|Einstellungen speichern|Diagnose starten|Neue Diagnose|Nicht geprüft|Test bestanden|Bestanden|Problem erkannt|Problem|Prüfung erforderlich|Abgebrochen|Nicht angegeben|Ja|Nein|Abbrechen|Bericht löschen|JSON exportieren|Diagnose suchen…|Alle anzeigen|Aktualisieren|Diagnose stoppen|Berichte vergleichen|Berichte auswählen|Schnellprüfung|Standard|Ausführlich',
  it: 'Panoramica|Diagnostica|Memoria RAM|Archiviazione|Processore|Cronologia|Confronto|Preferenze|Informazioni su Computy|Lingua dell’interfaccia|Sistema|Chiaro|Scuro|Segui il sistema|Salva preferenze|Avvia diagnostica|Nuova diagnostica|Non verificato|Test superato|Superato|Problema rilevato|Problema|Da esaminare|Interrotto|Non comunicato|Sì|No|Annulla|Elimina rapporto|Esporta JSON|Cerca una diagnostica…|Mostra tutto|Aggiorna|Interrompi diagnostica|Confronta rapporti|Scegli rapporti|Controllo rapido|Essenziale|Approfondito',
  nl: 'Overzicht|Diagnostiek|Werkgeheugen|Opslag|Processor|Geschiedenis|Vergelijking|Voorkeuren|Over Computy|Interfacetaal|Systeem|Licht|Donker|Systeem volgen|Voorkeuren opslaan|Diagnose starten|Nieuwe diagnose|Niet gecontroleerd|Test geslaagd|Geslaagd|Probleem gedetecteerd|Probleem|Nader bekijken|Onderbroken|Niet opgegeven|Ja|Nee|Annuleren|Rapport verwijderen|JSON exporteren|Diagnose zoeken…|Alles bekijken|Vernieuwen|Diagnose stoppen|Rapporten vergelijken|Rapporten kiezen|Snelle controle|Standaard|Uitgebreid',
  pl: 'Przegląd|Diagnostyka|Pamięć RAM|Dyski|Procesor|Historia|Porównanie|Preferencje|O Computy|Język interfejsu|System|Jasny|Ciemny|Zgodnie z systemem|Zapisz preferencje|Uruchom diagnostykę|Nowa diagnostyka|Nie sprawdzono|Test zaliczony|Zaliczony|Wykryto problem|Problem|Wymaga sprawdzenia|Przerwano|Nie podano|Tak|Nie|Anuluj|Usuń raport|Eksportuj JSON|Szukaj diagnostyki…|Pokaż wszystko|Odśwież|Zatrzymaj diagnostykę|Porównaj raporty|Wybierz raporty|Szybki test|Podstawowy|Szczegółowy',
  ru: 'Обзор|Диагностика|Оперативная память|Накопители|Процессор|История|Сравнение|Настройки|О Computy|Язык интерфейса|Системная|Светлая|Тёмная|Как в системе|Сохранить настройки|Запустить диагностику|Новая диагностика|Не проверено|Тест пройден|Пройден|Обнаружена проблема|Проблема|Требует проверки|Прервано|Нет данных|Да|Нет|Отмена|Удалить отчёт|Экспорт JSON|Найти диагностику…|Показать всё|Обновить|Остановить диагностику|Сравнить отчёты|Выбрать отчёты|Быстрая проверка|Основной|Подробный',
  uk: 'Огляд|Діагностика|Оперативна пам’ять|Накопичувачі|Процесор|Історія|Порівняння|Налаштування|Про Computy|Мова інтерфейсу|Системна|Світла|Темна|Як у системі|Зберегти налаштування|Запустити діагностику|Нова діагностика|Не перевірено|Тест пройдено|Пройдено|Виявлено проблему|Проблема|Потребує перевірки|Перервано|Дані відсутні|Так|Ні|Скасувати|Видалити звіт|Експорт JSON|Знайти діагностику…|Показати все|Оновити|Зупинити діагностику|Порівняти звіти|Вибрати звіти|Швидка перевірка|Основний|Докладний',
  tr: 'Genel bakış|Tanılama|RAM belleği|Depolama|İşlemci|Geçmiş|Karşılaştırma|Tercihler|Computy hakkında|Arayüz dili|Sistem|Açık|Koyu|Sistemi izle|Tercihleri kaydet|Tanılamayı başlat|Yeni tanılama|Doğrulanmadı|Test başarılı|Başarılı|Sorun algılandı|Sorun|İncelenmeli|Durduruldu|Bildirilmedi|Evet|Hayır|İptal|Raporu sil|JSON dışa aktar|Tanılama ara…|Tümünü gör|Yenile|Tanılamayı durdur|Raporları karşılaştır|Raporları seç|Hızlı kontrol|Temel|Ayrıntılı',
  ar: 'نظرة عامة|التشخيص|ذاكرة RAM|التخزين|المعالج|السجل|المقارنة|التفضيلات|حول Computy|لغة الواجهة|النظام|فاتح|داكن|اتباع النظام|حفظ التفضيلات|بدء التشخيص|تشخيص جديد|لم يتم التحقق|اجتاز الاختبار|اجتاز|تم اكتشاف مشكلة|مشكلة|يحتاج إلى مراجعة|تم الإيقاف|غير متاح|نعم|لا|إلغاء|حذف التقرير|تصدير JSON|البحث عن تشخيص…|عرض الكل|تحديث|إيقاف التشخيص|مقارنة التقارير|اختيار التقارير|فحص سريع|أساسي|فحص مفصّل',
  hi: 'अवलोकन|निदान|RAM मेमोरी|स्टोरेज|प्रोसेसर|इतिहास|तुलना|प्राथमिकताएँ|Computy के बारे में|इंटरफ़ेस की भाषा|सिस्टम|हल्का|गहरा|सिस्टम के अनुसार|प्राथमिकताएँ सहेजें|निदान शुरू करें|नया निदान|जाँच नहीं हुई|परीक्षण सफल|सफल|समस्या मिली|समस्या|समीक्षा आवश्यक|रोका गया|जानकारी उपलब्ध नहीं|हाँ|नहीं|रद्द करें|रिपोर्ट मिटाएँ|JSON निर्यात करें|निदान खोजें…|सभी देखें|रीफ़्रेश करें|निदान रोकें|रिपोर्टों की तुलना करें|रिपोर्ट चुनें|त्वरित जाँच|आवश्यक|विस्तृत',
  bn: 'সারসংক্ষেপ|পরীক্ষা|RAM মেমরি|স্টোরেজ|প্রসেসর|ইতিহাস|তুলনা|পছন্দসমূহ|Computy সম্পর্কে|ইন্টারফেসের ভাষা|সিস্টেম|হালকা|গাঢ়|সিস্টেম অনুসরণ করুন|পছন্দ সেভ করুন|পরীক্ষা শুরু করুন|নতুন পরীক্ষা|যাচাই করা হয়নি|পরীক্ষায় উত্তীর্ণ|উত্তীর্ণ|সমস্যা পাওয়া গেছে|সমস্যা|পর্যালোচনা প্রয়োজন|থামানো হয়েছে|তথ্য পাওয়া যায়নি|হ্যাঁ|না|বাতিল|রিপোর্ট মুছুন|JSON এক্সপোর্ট|পরীক্ষা খুঁজুন…|সব দেখুন|রিফ্রেশ|পরীক্ষা থামান|রিপোর্ট তুলনা করুন|রিপোর্ট বেছে নিন|দ্রুত পরীক্ষা|মৌলিক|বিস্তারিত',
  ur: 'جائزہ|تشخیص|RAM میموری|اسٹوریج|پروسیسر|تاریخچہ|موازنہ|ترجیحات|Computy کے بارے میں|انٹرفیس کی زبان|سسٹم|روشن|تاریک|سسٹم کی پیروی|ترجیحات محفوظ کریں|تشخیص شروع کریں|نئی تشخیص|جانچ نہیں ہوئی|ٹیسٹ کامیاب|کامیاب|مسئلہ دریافت ہوا|مسئلہ|جائزہ ضروری|روک دیا گیا|معلومات دستیاب نہیں|ہاں|نہیں|منسوخ|رپورٹ حذف کریں|JSON برآمد کریں|تشخیص تلاش کریں…|سب دیکھیں|تازہ کریں|تشخیص روکیں|رپورٹس کا موازنہ|رپورٹس منتخب کریں|فوری جانچ|بنیادی|تفصیلی',
  id: 'Ringkasan|Diagnostik|Memori RAM|Penyimpanan|Prosesor|Riwayat|Perbandingan|Preferensi|Tentang Computy|Bahasa antarmuka|Sistem|Terang|Gelap|Ikuti sistem|Simpan preferensi|Mulai diagnostik|Diagnostik baru|Belum diperiksa|Lulus pengujian|Lulus|Masalah terdeteksi|Masalah|Perlu ditinjau|Dihentikan|Tidak tersedia|Ya|Tidak|Batal|Hapus laporan|Ekspor JSON|Cari diagnostik…|Lihat semua|Segarkan|Hentikan diagnostik|Bandingkan laporan|Pilih laporan|Pemeriksaan cepat|Dasar|Mendalam',
  vi: 'Tổng quan|Chẩn đoán|Bộ nhớ RAM|Lưu trữ|Bộ xử lý|Lịch sử|So sánh|Tùy chọn|Giới thiệu Computy|Ngôn ngữ giao diện|Hệ thống|Sáng|Tối|Theo hệ thống|Lưu tùy chọn|Bắt đầu chẩn đoán|Chẩn đoán mới|Chưa kiểm tra|Đạt kiểm tra|Đạt|Phát hiện sự cố|Sự cố|Cần xem xét|Đã dừng|Không có dữ liệu|Có|Không|Hủy|Xóa báo cáo|Xuất JSON|Tìm chẩn đoán…|Xem tất cả|Làm mới|Dừng chẩn đoán|So sánh báo cáo|Chọn báo cáo|Kiểm tra nhanh|Cơ bản|Chuyên sâu',
  th: 'ภาพรวม|การวินิจฉัย|หน่วยความจำ RAM|ที่เก็บข้อมูล|หน่วยประมวลผล|ประวัติ|เปรียบเทียบ|การตั้งค่า|เกี่ยวกับ Computy|ภาษาของอินเทอร์เฟซ|ระบบ|สว่าง|มืด|ตามระบบ|บันทึกการตั้งค่า|เริ่มการวินิจฉัย|การวินิจฉัยใหม่|ยังไม่ได้ตรวจสอบ|ผ่านการทดสอบ|ผ่าน|พบปัญหา|ปัญหา|ควรตรวจสอบ|หยุดแล้ว|ไม่มีข้อมูล|ใช่|ไม่|ยกเลิก|ลบรายงาน|ส่งออก JSON|ค้นหาการวินิจฉัย…|ดูทั้งหมด|รีเฟรช|หยุดการวินิจฉัย|เปรียบเทียบรายงาน|เลือกรายงาน|ตรวจสอบด่วน|พื้นฐาน|เชิงลึก',
  ko: '개요|진단|RAM 메모리|저장 장치|프로세서|기록|비교|환경 설정|Computy 정보|인터페이스 언어|시스템|밝게|어둡게|시스템 설정 따르기|설정 저장|진단 시작|새 진단|확인하지 않음|검사 통과|통과|문제 감지|문제|검토 필요|중단됨|정보 없음|예|아니요|취소|보고서 삭제|JSON 내보내기|진단 검색…|모두 보기|새로 고침|진단 중지|보고서 비교|보고서 선택|빠른 검사|기본|상세',
  ja: '概要|診断|RAMメモリ|ストレージ|プロセッサ|履歴|比較|設定|Computyについて|表示言語|システム|ライト|ダーク|システムに合わせる|設定を保存|診断を開始|新しい診断|未確認|テスト合格|合格|問題を検出|問題|要確認|中断|情報なし|はい|いいえ|キャンセル|レポートを削除|JSONをエクスポート|診断を検索…|すべて表示|更新|診断を停止|レポートを比較|レポートを選択|クイックチェック|標準|詳細',
  'zh-CN':
    '概览|诊断|内存|存储|处理器|历史记录|对比|偏好设置|关于 Computy|界面语言|系统|浅色|深色|跟随系统|保存设置|开始诊断|新建诊断|尚未验证|测试通过|通过|检测到问题|问题|需要检查|已中断|未提供|是|否|取消|删除报告|导出 JSON|搜索诊断…|查看全部|刷新|停止诊断|对比报告|选择报告|快速检查|标准|深入检查',
  'zh-TW':
    '總覽|診斷|記憶體|儲存裝置|處理器|歷史記錄|比較|偏好設定|關於 Computy|介面語言|系統|淺色|深色|跟隨系統|儲存設定|開始診斷|新增診斷|尚未驗證|測試通過|通過|偵測到問題|問題|需要檢查|已中斷|未提供|是|否|取消|刪除報告|匯出 JSON|搜尋診斷…|查看全部|重新整理|停止診斷|比較報告|選擇報告|快速檢查|標準|深入檢查',
  fa: 'نمای کلی|عیب‌یابی|حافظهٔ RAM|ذخیره‌سازی|پردازنده|تاریخچه|مقایسه|تنظیمات|دربارهٔ Computy|زبان رابط کاربری|سیستم|روشن|تیره|پیروی از سیستم|ذخیرهٔ تنظیمات|شروع عیب‌یابی|عیب‌یابی جدید|بررسی نشده|آزمون موفق|موفق|مشکل شناسایی شد|مشکل|نیاز به بررسی|متوقف شد|اطلاعات موجود نیست|بله|خیر|لغو|حذف گزارش|خروجی JSON|جستجوی عیب‌یابی…|نمایش همه|تازه‌سازی|توقف عیب‌یابی|مقایسهٔ گزارش‌ها|انتخاب گزارش‌ها|بررسی سریع|پایه|بررسی دقیق',
  he: 'סקירה|אבחון|זיכרון RAM|אחסון|מעבד|היסטוריה|השוואה|העדפות|אודות Computy|שפת הממשק|מערכת|בהיר|כהה|לפי המערכת|שמירת העדפות|התחלת אבחון|אבחון חדש|לא נבדק|הבדיקה עברה|עבר|זוהתה בעיה|בעיה|דורש בדיקה|הופסק|לא נמסר|כן|לא|ביטול|מחיקת דוח|ייצוא JSON|חיפוש אבחון…|הצגת הכול|רענון|עצירת אבחון|השוואת דוחות|בחירת דוחות|בדיקה מהירה|בסיסי|מעמיק',
  sw: 'Muhtasari|Uchunguzi|Kumbukumbu ya RAM|Hifadhi|Kichakataji|Historia|Ulinganisho|Mapendeleo|Kuhusu Computy|Lugha ya kiolesura|Mfumo|Mwanga|Giza|Fuata mfumo|Hifadhi mapendeleo|Anza uchunguzi|Uchunguzi mpya|Haijathibitishwa|Jaribio limefaulu|Imefaulu|Tatizo limegunduliwa|Tatizo|Inahitaji ukaguzi|Imesitishwa|Haijatolewa|Ndiyo|Hapana|Ghairi|Futa ripoti|Hamisha JSON|Tafuta uchunguzi…|Tazama zote|Onyesha upya|Simamisha uchunguzi|Linganisha ripoti|Chagua ripoti|Ukaguzi wa haraka|Msingi|Kwa kina',
};
const output = {};
for (const [language, row] of Object.entries(rows)) {
  const values = row.split('|');
  if (values.length !== keys.length)
    throw new Error(`${language}: ${values.length} instead of ${keys.length}`);
  output[language] = Object.fromEntries(keys.map((key, index) => [key, values[index]]));
}
Object.assign(output.en, {
  'Votre matériel, en clair.': 'Your hardware, clearly.',
  'VOTRE MATÉRIEL, EN CLAIR': 'YOUR HARDWARE, CLEARLY',
  'Un diagnostic qui a du sens.': 'Diagnostics that make sense.',
  'L’histoire de votre matériel.': 'Your hardware has a history.',
  'Les changements, côte à côte.': 'Changes, side by side.',
  'Un espace qui vous ressemble.': 'Make yourself at home.',
  'Un check-up.': 'One check-up.',
  'L’esprit plus léger': 'A little peace of mind',
  'Vos contrôles sont conformes': 'Your checks passed',
  'Aucun contrôle exploitable': 'No usable checks',
  'Votre PC. Vos données.': 'Your PC. Your data.',
  'Aucune télémétrie': 'No telemetry',
  'Moteur Rust connecté': 'Rust engine connected',
  'Conçu par': 'Made by',
  'Motifs, inversions mobiles et compteurs ECC accessibles.':
    'Patterns, moving inversions and available ECC counters.',
  'Calculs parallèles comparés à des réponses connues.':
    'Parallel calculations checked against known answers.',
  'SMART / NVMe et intégrité d’un fichier temporaire.':
    'SMART / NVMe and temporary-file integrity.',
  'Ce qu’on peut vraiment vérifier': 'What we can actually verify',
  'Note des contrôles': 'Check score',
  diagnostics: 'diagnostics',
  diagnostic: 'diagnostic',
  sélectionné: 'selected',
  enregistré: 'saved',
  passe: 'pass',
  secondes: 'seconds',
  Mio: 'MiB',
  Gio: 'GiB',
  'S’adapte automatiquement': 'Automatically adapts',
  'Les traductions sont incluses dans l’application. Aucun service de traduction n’est appelé.':
    'Translations are included in the app. No translation service is contacted.',
  '25 langues · dates et nombres localisés · langues de droite à gauche prises en charge':
    '25 languages · local dates and numbers · right-to-left support',
});
const overviewKeys = [
  'Votre matériel, en clair.',
  'Un check-up.',
  'L’esprit plus léger',
  'RAM, disques, processeur : faites le point',
  'sur ce qui fait tourner votre quotidien.',
  '{{v0}} cœurs',
  'cœurs physiques ·',
];
const overviewRows = {
  en: 'Your hardware, clearly.|One check-up.|A little peace of mind|RAM, drives, processor: check the components|that keep your everyday life running.|{{v0}} cores|physical cores ·',
  es: 'Tu hardware, sin misterios.|Una revisión.|Más tranquilidad|RAM, discos y procesador: revisa los componentes|que hacen funcionar tu día a día.|{{v0}} núcleos|núcleos físicos ·',
  'pt-BR':
    'Seu hardware, com clareza.|Uma verificação.|Mais tranquilidade|RAM, discos e processador: confira os componentes|que fazem parte do seu dia a dia.|{{v0}} núcleos|núcleos físicos ·',
  de: 'Ihre Hardware, verständlich.|Ein Check-up.|Ein gutes Gefühl|RAM, Laufwerke, Prozessor: Prüfen Sie die Komponenten,|die Ihren Alltag am Laufen halten.|{{v0}} Kerne|physische Kerne ·',
  it: 'Il tuo hardware, in chiaro.|Un controllo.|Più tranquillità|RAM, dischi e processore: controlla i componenti|che fanno funzionare la tua quotidianità.|{{v0}} core|core fisici ·',
  nl: 'Je hardware, helder uitgelegd.|Een controle.|Meer gemoedsrust|RAM, schijven en processor: bekijk de onderdelen|die je dagelijks gebruikt.|{{v0}} kernen|fysieke kernen ·',
  pl: 'Twój sprzęt bez tajemnic.|Jeden przegląd.|Więcej spokoju|RAM, dyski, procesor: sprawdź podzespoły,|z których korzystasz każdego dnia.|Rdzenie: {{v0}}|rdzenie fizyczne ·',
  ru: 'Ваше оборудование — понятно.|Одна проверка.|Больше спокойствия|Память, диски, процессор: проверьте компоненты,|которые помогают вам каждый день.|Ядра: {{v0}}|физические ядра ·',
  uk: 'Ваше обладнання — зрозуміло.|Одна перевірка.|Більше спокою|Пам’ять, диски, процесор: перевірте компоненти,|які допомагають вам щодня.|Ядра: {{v0}}|фізичні ядра ·',
  tr: 'Donanımınız, anlaşılır biçimde.|Bir kontrol.|Daha fazla huzur|RAM, diskler ve işlemci: günlük işlerinizi|yürüten bileşenleri kontrol edin.|{{v0}} çekirdek|fiziksel çekirdek ·',
  ar: 'مكوّنات جهازك، بوضوح.|فحص واحد.|مزيد من الاطمئنان|الذاكرة والأقراص والمعالج: افحص المكوّنات|التي تعتمد عليها كل يوم.|{{v0}} نواة|أنوية فعلية ·',
  hi: 'आपके हार्डवेयर की साफ़ जानकारी।|एक जाँच।|थोड़ा सुकून|RAM, डिस्क और प्रोसेसर: उन हिस्सों की जाँच करें|जिन पर आपका रोज़ का काम निर्भर है।|{{v0}} कोर|भौतिक कोर ·',
  bn: 'আপনার হার্ডওয়্যারের পরিষ্কার তথ্য।|একটি পরীক্ষা।|একটু স্বস্তি|RAM, ডিস্ক ও প্রসেসর: সেই যন্ত্রাংশগুলি পরীক্ষা করুন|যেগুলির উপর আপনার দৈনন্দিন কাজ নির্ভর করে।|{{v0}} কোর|ফিজিক্যাল কোর ·',
  ur: 'آپ کے ہارڈویئر کی واضح معلومات۔|ایک جانچ۔|کچھ اطمینان|RAM، ڈسک اور پروسیسر: ان حصوں کی جانچ کریں|جن پر آپ کا روزمرہ کام منحصر ہے۔|{{v0}} کور|فزیکل کور ·',
  id: 'Perangkat keras Anda, lebih jelas.|Satu pemeriksaan.|Lebih tenang|RAM, disk, prosesor: periksa komponen|yang mendukung aktivitas sehari-hari Anda.|{{v0}} inti|inti fisik ·',
  vi: 'Hiểu rõ phần cứng của bạn.|Một lần kiểm tra.|An tâm hơn|RAM, ổ đĩa, bộ xử lý: kiểm tra các linh kiện|giúp công việc hằng ngày của bạn vận hành.|{{v0}} nhân|nhân vật lý ·',
  th: 'เข้าใจฮาร์ดแวร์ของคุณอย่างชัดเจน|ตรวจสอบสักครั้ง|สบายใจขึ้น|RAM ดิสก์ และหน่วยประมวลผล: ตรวจสอบส่วนประกอบ|ที่ช่วยให้คุณทำงานได้ทุกวัน|{{v0}} คอร์|คอร์จริง ·',
  ko: '하드웨어를 한눈에.|한 번의 점검.|더 편안한 마음으로|RAM, 디스크, 프로세서: 매일 사용하는|컴퓨터의 부품을 확인하세요.|{{v0}}코어|물리 코어 ·',
  ja: 'パソコンの状態を、わかりやすく。|ひとつの点検で。|もっと安心を|RAM、ディスク、プロセッサ。毎日を支える|パソコンの部品を確認しましょう。|{{v0}}コア|物理コア ·',
  'zh-CN':
    '硬件状态，一目了然。|检查一下。|用得更安心|内存、磁盘、处理器：检查这些|支持日常工作的电脑组件。|{{v0}} 个核心|物理核心 ·',
  'zh-TW':
    '硬體狀態，一目瞭然。|檢查一下。|用得更安心|記憶體、磁碟、處理器：檢查這些|支援日常工作的電腦零組件。|{{v0}} 個核心|實體核心 ·',
  fa: 'سخت‌افزار شما، روشن و ساده.|یک بررسی.|آسودگی بیشتر|حافظه، دیسک و پردازنده: قطعاتی را بررسی کنید|که هر روز به آن‌ها تکیه می‌کنید.|{{v0}} هسته|هسته‌های فیزیکی ·',
  he: 'החומרה שלך, בצורה ברורה.|בדיקה אחת.|יותר שקט נפשי|זיכרון, כוננים ומעבד: בדקו את הרכיבים|שעליהם אתם מסתמכים מדי יום.|{{v0}} ליבות|ליבות פיזיות ·',
  sw: 'Vifaa vyako, kwa uwazi.|Ukaguzi mmoja.|Utulivu zaidi|RAM, diski na kichakataji: kagua vifaa|vinavyosaidia shughuli zako za kila siku.|Viini {{v0}}|viini halisi ·',
};
for (const [language, row] of Object.entries(overviewRows)) {
  const values = row.split('|');
  if (values.length !== overviewKeys.length) throw new Error(`Invalid overview row: ${language}`);
  Object.assign(
    output[language],
    Object.fromEntries(overviewKeys.map((key, index) => [key, values[index]])),
    { LOCAL: 'LOCAL' },
  );
}
Object.assign(output.en, {
  'Moteur Rust · schéma v': 'Report format v',
  'Actualiser les informations': 'Refresh hardware info',
  'Session approfondie': 'In-depth session test',
  'Dans ma session': 'In my session',
  'Avant démarrage': 'Before system startup',
  'Sans clé USB': 'Without a USB drive',
  'Avec clé USB': 'With a USB drive',
  'Aucune erreur détectée': 'No errors detected',
  'Informations indisponibles, pas erreurs détectées': 'Missing information, not detected errors',
  'Non vérifiés : {{count}}': 'Unavailable: {{count}}',
  'Préparer le prochain démarrage': 'Prepare the next boot',
  'Tester la mémoire avant le lancement du système': 'Test memory before the system starts',
});
writeFileSync('src/locales/overrides.json', JSON.stringify(output, null, 2) + '\n');
console.log(
  `${keys.length} reviewed navigation, action and result labels across ${Object.keys(rows).length} translated locales.`,
);
