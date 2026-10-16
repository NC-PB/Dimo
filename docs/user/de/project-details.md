# Projekt- und Berichtsdaten

**Datei > Info** ist der eine Ort, an dem du die Daten änderst, die Teil, Zeichnung und
Prüfbericht kennzeichnen. Ist nichts ausgewählt, zeigt der Eigenschaftenbereich eine Übersicht mit
**Unter Datei > Info bearbeiten**, und der Reiter Bericht verweist dorthin, wenn ein Feld im
Berichtskopf leer ist. Die Daten liegen im Projekt und stehen in Kopfzeile und Unterschriftsblock der
Berichte.

Jedes Feld wird gespeichert, wenn du es verlässt oder Enter drückst. Jede Änderung ist ein Schritt
beim Rückgängigmachen (⌘Z), wie jede andere Änderung am Projekt. Lehnt DimoMark einen Wert ab, zeigt das
Feld wieder den alten Wert, und eine Meldung über der Zeichnung nennt den Grund.

## Teil

| Feld | Bedeutung |
|---|---|
| Teilenummer | Nummer des Teils, wie der Kunde sie nennt |
| Teilebezeichnung | Name des Teils |
| Kunde | Wer das Teil bestellt hat |
| Auftragsnummer | Auftrags- oder Bestellbezug |

## Zeichnung

| Feld | Bedeutung |
|---|---|
| Änderungsstand | Änderungsstand der Zeichnung, zum Beispiel `B`. DimoMark liest ihn nicht aus dem Schriftfeld, tippe ihn von der Zeichnung ab |
| Zeichnungsdatei | Dateiname der importierten Zeichnung (nicht bearbeitbar) |
| SHA-256 | Fingerabdruck der Zeichnungsdatei (nicht bearbeitbar). **Kopieren** legt ihn in die Zwischenablage. Der Bericht druckt ihn, so lässt sich prüfen, dass ein Bericht zu genau dieser Zeichnung gehört |

Der Änderungsstand gehört zur Zeichnung im Projekt. DimoMark speichert ihn mit dem Projekt und druckt
ihn auf jeden Bericht.

## Bericht

| Feld | Bedeutung |
|---|---|
| Berichtsnummer | Nummer des Prüfberichts |
| Berichtsdatum | Datum des Berichts. Lass es leer, dann trägt der erste Berichtsexport das heutige Datum ein; oder wähle **Heute**, um es jetzt einzutragen, oder tippe ein Datum. Einmal gespeichert, gehört das Datum zum Projekt und ändert sich nicht von selbst |
| Firma | Name deiner Firma. Siehe unten |
| Geprüft von | Wer die Teile geprüft hat |
| Freigegeben von | Wer den Bericht freigegeben hat |
| Bemerkungen | Freier Text, der auf dem Bericht steht |

„Geprüft von“ und „Freigegeben von“ stehen im Unterschriftenblock des Prüfberichts. Das Datum bei
„Geprüft von“ ist das Berichtsdatum; „Freigegeben von“ bekommt kein Datum, das schreibst du von
Hand. Das ist getrennt vom Prüfer jedes Teils, den du im Eigenschaftenbereich des Reiters Messen einstellst
([Ergebnisse eingeben](measuring.md)). Ist das Berichtsdatum beim Export eines Prüfblatts oder
Prüfberichts leer, trägt DimoMark zuerst das heutige Datum ein, als eine Änderung, die du rückgängig
machen kannst ([Exporte](exports.md)).

Leere Felder bleiben auf dem Bericht leer. Ein neues Projekt beginnt mit dem Firmennamen aus den **Optionen** und mit dem Benutzernamen der **Optionen** (oder deines Systems) als Prüfer. DimoMark ändert ein bestehendes Projekt nicht, wenn du es öffnest.

### Firmenname

Trage deinen Firmennamen einmal unter **Datei** > **Optionen** bei **Du und deine Firma** ein
([Datei-Seite, Optionen und Über DimoMark](settings.md)). Ein neues Projekt beginnt mit diesem Namen im Feld, als Teil seines
ersten Zustands (kein Rückgängig-Schritt, das Projekt gilt nicht als geändert). Eine spätere
Änderung der Einstellung ändert bestehende Projekte nicht; du kannst das Feld in jedem Projekt
bearbeiten.
