// Northview Dental new-patient form. Filled by pdf.rs with sys.inputs.data (JSON).
#let d = json(bytes(sys.inputs.data))

#let ink = rgb("#212633")
#let muted = rgb("#5F6673")
#let blue = rgb("#066FAD")
#let red = rgb("#B42318")
#let redbg = rgb("#FCECEC")
#let line-c = rgb("#DDE4EA")

#set document(title: d.name + " — New Patient Form", author: "Northview Dental")
#set page(paper: "us-letter", margin: (x: 0.85in, top: 0.75in, bottom: 0.8in),
  footer: context [
    #set text(8pt, fill: muted)
    #d.name · #d.ref #h(1fr) Page #counter(page).display() of #counter(page).final().first()
  ])
#set text(font: "Poppins", size: 10pt, fill: ink)
#set par(leading: 0.6em)

#grid(columns: (1fr, auto), align: (left + horizon, right + horizon),
  [
    #image("logo.png", height: 30pt)
    #v(2pt)
    #text(8pt, fill: muted)[#d.practice_address · #d.practice_phone]
  ],
  [
    #text(weight: "semibold")[#d.ref] \
    #text(8pt, fill: muted)[Submitted #d.submitted]
  ],
)
#v(4pt)
#line(length: 100%, stroke: 1.2pt + ink)
#v(10pt)
#text(18pt, weight: "semibold")[New Patient Form]
#v(6pt)

#let section(title) = {
  v(8pt)
  text(8pt, weight: "semibold", fill: blue, tracking: 0.08em)[#upper(title)]
  v(-2pt)
}

#let row(label, value, warn: false) = {
  let v = if value == "" { text(fill: muted)[—] } else { value }
  if warn {
    block(fill: redbg, radius: 4pt, inset: (x: 6pt, y: 5pt), width: 100%, below: 0pt,
      grid(columns: (1.6in, 1fr), text(fill: red)[#label], text(fill: red, weight: "semibold")[#v]))
  } else {
    block(inset: (x: 6pt, y: 5pt), width: 100%, below: 0pt, stroke: (bottom: 0.5pt + line-c),
      grid(columns: (1.6in, 1fr), text(fill: muted)[#label], v))
  }
}

#section("Patient")
#row("Name", d.name)
#row("Date of birth", d.dob)
#row("Phone", d.phone)
#row("Email", d.email)
#row("Address", d.address)
#row("First visit", d.visit)

#section("Medical history")
#row("Conditions", d.conditions)
#row("Medications", d.medications)
#row("Allergies", d.allergies, warn: d.allergy)
#row("Last dental visit", d.last_dental)

#section("Insurance")
#row("Carrier", d.carrier)
#row("Member ID", d.member_id)

#section("Signature")
#block(inset: (x: 6pt, y: 8pt), width: 100%)[
  #text(16pt, style: "italic")[#d.signature]
  #v(-6pt)
  #line(length: 3in, stroke: 0.5pt + muted)
  #text(8pt, fill: muted)[Signed electronically by #d.signature · #d.submitted]
]
