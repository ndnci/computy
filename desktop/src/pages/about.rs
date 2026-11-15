//! About Computy and the diagnostic method (former pages/About.tsx).
use crate::{
    app::Computy,
    components::{self, ButtonKind, NoticeTone, button_with, logo, notice},
    i18n::tx,
    platform,
    theme::{self, c, hex},
    ui::{icon, panel, spaced, text},
};
use gpui::*;

fn link(
    id: &'static str,
    label: &str,
    url: &'static str,
    color: Hsla,
    weight: f32,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(6.))
        .cursor_pointer()
        .child(text(tx(label).trim().to_owned(), 11., weight, color))
        .child(icon("arrow-up-right", 13., "2", color))
        .on_click(move |_, _, cx| platform::open_external(url, cx))
}

fn paragraph(content: &str) -> Div {
    text(tx(content), 12., 400., c((0x7d9269, 0xadc29a))).line_height(px(22.8))
}

fn value(icon_name: &str, title: &str, body: &str) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .p(px(22.))
        .bg(c(theme::PANEL))
        .border_1()
        .border_color(c(theme::BORDER))
        .rounded(px(13.))
        .flex()
        .flex_col()
        .child(icon(icon_name, 23., "2", hex(0x9eb780)))
        .child(spaced(tx(title), 14., 650., c(theme::HEADING), -0.2, 18.).mt(px(16.)))
        .child(paragraph(body).mt(px(10.)))
}

fn method(
    number: &str,
    title: &str,
    paragraphs: &[&str],
    reference: Option<(&'static str, &str, &'static str)>,
    last: bool,
) -> impl IntoElement {
    let mut body = div().flex_1().min_w_0().flex().flex_col().child(spaced(
        tx(title),
        14.,
        650.,
        c(theme::HEADING),
        -0.2,
        18.,
    ));
    for p in paragraphs {
        body = body.child(paragraph(p).mt(px(11.)));
    }
    if let Some((id, label, url)) = reference {
        body = body.child(div().mt(px(15.)).flex().child(link(
            id,
            label,
            url,
            hex(0x87a56b),
            400.,
        )));
    }
    div()
        .flex()
        .gap(px(19.))
        .py(px(23.))
        .when(!last, |d| d.border_b_1().border_color(c(theme::BORDER)))
        .child(
            div()
                .pt(px(2.))
                .child(text(tx(number), 10., 400., hex(0xa4b790))),
        )
        .child(body)
}

trait When: Sized {
    fn when(self, condition: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if condition { f(self) } else { self }
    }
}
impl When for Div {}

fn rule(score: &str, small: bool, color: u32, title: &str, body: &str) -> impl IntoElement {
    let mut value =
        div()
            .flex()
            .items_baseline()
            .child(spaced(tx(score), 24., 550., hex(color), -0.6, 31.));
    if small {
        value = value.child(text(tx("/10"), 10., 400., hex(color)));
    }
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(value)
        .child(text(tx(title), 12., 550., c(theme::TEXT)).mt(px(13.)))
        .child(
            text(tx(body), 11., 400., hex(0xa8b696))
                .line_height(px(20.9))
                .mt(px(9.)),
        )
}

pub fn render(_app: &mut Computy, _window: &mut Window, cx: &mut Context<Computy>) -> AnyElement {
    let _ = cx;
    div()
        .flex()
        .flex_col()
        .child(components::section_title(
            Some("UNE AUTRE FAÇON DE PRENDRE SOIN DE SON PC"),
            "Moins de mystère. Plus de clarté.",
            Some("Computy rend les diagnostics matériels accessibles, sans cacher leurs limites."),
            None,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(38.))
                .p(px(35.))
                .mb(px(24.))
                .bg(c((0xedf4e3, 0x2c4028)))
                .border_1()
                .border_color(c((0xdce7cc, 0x405531)))
                .rounded(px(17.))
                .child(div().ml(px(1.)).child(logo(138.)))
                .child(
                    div()
                        .w(px(450.))
                        .flex()
                        .flex_col()
                        .child(spaced(format!("Computy · {} {}", tx("Version"), env!("CARGO_PKG_VERSION")), 10., 650., hex(0x899582), 1.75, 13.).mt(px(6.)))
                        .child(
                            div()
                                .mt(px(14.))
                                .flex()
                                .flex_col()
                                .child(spaced(tx("Comprendre aujourd’hui."), 27., 650., c(theme::HEADING), -1., 37.8))
                                .child(spaced(tx("Mieux choisir demain."), 27., 650., c(theme::HEADING), -1., 37.8)),
                        )
                        .child(paragraph("Un logiciel de bureau pour observer votre matériel, vérifier son comportement et garder une trace de ses évolutions.").mt(px(14.)))
                        .child(
                            div().mt(px(23.)).flex().child(
                                button_with("about-ndnci", ButtonKind::Primary, false, None, tx("Une création NDNCI ").trim().to_owned(), false)
                                    .child(icon("arrow-up-right", 16., "2", hex(0xffffff)))
                                    .on_click(|_, _, cx| platform::open_external("https://www.ndnci.com", cx)),
                            ),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .gap(px(20.))
                .mb(px(24.))
                .child(value("code-2", "Un moteur natif en Rust", "Calculs, lecture du matériel et diagnostics dans Rust. L’interface native est dessinée avec GPUI, sans moteur web ni Chromium."))
                .child(value("lock-keyhole", "Local, par conception", "Les diagnostics et leur historique restent sur votre machine. Vous décidez quand exporter un rapport ou ouvrir une ressource externe."))
                .child(value("shield-check", "Des limites explicites", "Une donnée indisponible reste inconnue. Un contrôle interrompu reste incomplet. Chaque résultat conserve ses mesures et sa portée.")),
        )
        .child(
            panel()
                .mb(px(20.))
                .flex()
                .flex_col()
                .child(
                    components::panel_header("Notre méthode de diagnostic", Some(tx("Ce qui est contrôlé, et ce qui ne peut pas être certifié.")), None)
                        .border_b_1()
                        .border_color(c(theme::BORDER)),
                )
                .child(
                    div()
                        .px(px(23.))
                        .flex()
                        .flex_col()
                        .child(method(
                            "01",
                            "Mémoire : écrire, relire, comparer",
                            &[
                                "Le moteur alloue une zone bornée et y écrit plusieurs motifs : adresses virtuelles, zéros et uns, damier, séquences pseudo-aléatoires et bits mobiles. Il vérifie puis inverse les mots dans les deux sens, avec des accès volatils pour empêcher la suppression des opérations par le compilateur.",
                                "Le verrouillage physique est tenté et son résultat est enregistré. Les caches restent actifs ; sans verrouillage, des pages peuvent être échangées sur le disque. Les compteurs ECC Linux sont lus lorsqu’ils sont accessibles.",
                            ],
                            Some(("ref-memtest", "Référence : philosophie et algorithmes de Memtest86+ ", "https://github.com/memtest86plus/memtest86plus#memory-testing-philosophy")),
                            false,
                        ))
                        .child(method(
                            "02",
                            "Stockage : télémétrie et intégrité",
                            &[
                                "Computy interroge smartctl en JSON et interprète son code de retour comme un masque de bits. Les alertes NVMe, les erreurs de média, les secteurs en attente et les historiques disponibles sont examinés. Les disques en veille peuvent rester non interrogés.",
                                "Un fichier dédié, créé sans écrasement, est rempli de motifs différents par bloc, synchronisé puis relu et comparé octet par octet. Il est ensuite supprimé. Ce contrôle porte sur le volume des données de Computy et ne constitue pas un test de tous les secteurs.",
                            ],
                            Some(("ref-smart", "Référence : smartmontools et smartctl ", "https://www.smartmontools.org/")),
                            false,
                        ))
                        .child(method(
                            "03",
                            "Processeur : des réponses connues",
                            &["Jusqu’à quatre threads calculent des SHA-256 d’un million de caractères « a » et des sommes entières, comparés à des réponses fixes connues. La durée, le nombre de calculs et les divergences sont enregistrés. Ce contrôle ne couvre pas toutes les instructions ni la stabilité prolongée."],
                            None,
                            true,
                        )),
                ),
        )
        .child(
            panel()
                .mb(px(24.))
                .flex()
                .flex_col()
                .child(components::panel_header("Une note lisible, jamais une garantie", None, None))
                .child(
                    div()
                        .px(px(23.))
                        .flex()
                        .gap(px(19.))
                        .child(rule("10", true, 0x82a965, "Conforme au contrôle", "Aucune anomalie détectée par le contrôle effectué."))
                        .child(rule("5", true, 0xb99b5a, "À examiner", "Usure ou historique d’erreurs nécessitant un examen."))
                        .child(rule("0", true, 0xbc7c6c, "Anomalie détectée", "Une erreur vérifiée ou une alerte matérielle est présente."))
                        .child(rule("—", false, 0xb0bda2, "Sans note", "Contrôle indisponible, absent ou interrompu.")),
                )
                .child(
                    div().p(px(23.)).child(notice(
                        NoticeTone::Info,
                        vec![tx("La note du bilan reprend la note la plus basse des contrôles exploitables. Les contrôles indisponibles réduisent la couverture et restent visibles. Un diagnostic interrompu n’obtient pas de note globale de réussite.").into_any_element()],
                    )),
                ),
        )
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(20.))
                .p(px(25.))
                .mb(px(35.))
                .bg(c((0xf0f4e9, 0x253322)))
                .border_1()
                .border_color(c((0xe0e8d4, 0x394a2e)))
                .rounded(px(13.))
                .child(icon("memory-stick", 24., "2", hex(0xa5b78c)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(spaced(tx("Pourquoi aucun logiciel sérieux ne promet « 100 % fiable »"), 14., 650., c(theme::HEADING), -0.2, 18.))
                        .child(paragraph("Un test lancé dans l’OS ne peut pas vérifier la mémoire occupée par le système. Même un test autonome peut manquer une panne intermittente, dépendante de la température ou d’une combinaison de composants. Un résultat correct signifie « aucune erreur observée dans ces conditions ».").mt(px(12.)))
                        .child(div().mt(px(18.)).flex().child(link("ref-duration", "Comprendre la durée et les pannes intermittentes ", "https://www.memtest86.com/tech_execution-time.html", hex(0x87a56b), 400.))),
                ),
        )
        .child(
            div()
                .flex()
                .justify_center()
                .items_center()
                .gap(px(4.))
                .child(text(tx("Pensé pour vos machines. Conçu avec soin par ").trim().to_owned(), 11., 400., hex(0xa6b795)).line_height(px(18.15)))
                .child(link("signature-ndnci", "NDNCI ", "https://www.ndnci.com", hex(0x91a97b), 600.)),
        )
        .into_any_element()
}
