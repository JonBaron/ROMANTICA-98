/*
 * ROMANTICA'98 invitasjonsintro (INVIT.EXE) - rekonstruert som pseudo-C.
 *
 * Originalen er håndskrevet 32-bit assembler (TASM32 + DLINK, DOS32 v3.5-extender),
 * med musikk via "USM Play v1.0a" / Useless Sound System av FreddyV/Useless.
 * Navn på variabler og funksjoner er gjettet; logikk, konstanter og
 * rekkefølge er hentet direkte fra den utpakkede binærkoden.
 * Hex-adressene i kommentarene er offset i det utpakkede programbildet.
 */

/* ---------- Data (offset 0x0000 - 0x2913) ---------- */
short SinAmp = 0xDFFF;            /* 0x0000  multiplikator for sinustabellen (signert: -8193) */
short SinTab[513];                /* 0x0002 */
short CosTab[513];                /* 0x0404 */

byte  *EarthTex;                  /* earth2.pcx   256 bred tekstur til tunnelen          */
byte  *TunnelTab;                 /* 200 x 256 x (u,v) forhåndsberegnet tunnel-tabell     */
byte  *FlowerGfx;                 /* blomster.pcx 450x85, 5 animasjonsrammer à 90 px     */
byte  *BannerGfx;                 /* banner2.pcx  320x40, toppbanner                      */
byte  *VScreen;                   /* 320x200 virtuell skjerm                              */
byte  *TextPages;                 /* 1600x200 = 5 "sider" med ferdig-rendret tekst        */
byte  *LogoMask;                  /* romintro.pcx 276x32, maske for "ROMANTICA'98"-logo   */
byte  *FontGfx;  int FontWidth;   /* font26.pcx   631x24 (632 byte per linje)             */
byte  IntroTex[256*256];          /* st_bak.pcx   256x256 (statisk buffer på 0x2913)      */
byte  IntroPal[768], MainPal[768];

int   Fade = 1;                   /* 0..255, lysstyrke på paletten                        */
int   Frame;                      /* teller / vinkel                                      */
int   EscTimer, QuitCount;
byte  LeftTimer, RightTimer;
int   FlowerFrame = 4;            /* 0=ESC, 1=venstre, 2=høyre, 4=hvile                  */
int   ScrollX, ScrollTarget;      /* myk horisontal scrolling mellom tekstsidene          */
short LogoWidth = 1;
short LogoWidths[12] = { 0, 22, 49, 80, 108, 141, 168, 176, 193, 220, 239, 276 };
                                  /* x-grensene for hver bokstav i ROMANTICA'98            */
byte  ShadeTab[256];
int   TextX, TextY;

char  Charset[]    = "abcdefghijklmnopqrstuvwxyz{}1234567890:.'/!-@";
char  ScrollText[] =
  "      {[invitation]}\\_you are hereby wellcome to:\\   romantica'98 !!\\_ date : 20-22 feb."
  "\\place : porsgrunn\\price : 200.-\\_arranged by tp n' dnf\\>"
  "        {[where]}\\_the party is located in\\porsgrunn in norway.\\vestsiden ungdumskole."
  "\\see roadmap on webpage\\or mail the organizers\\for further details.\\centrum 250m away.\\>"
  "       {[compos]}\\_appart from the normal\\democompos there will\\also be lotsa fun and"
  "\\fast compos. like\\swappercompo. ateam.\\coladrinking. joik.\\live codecompo. etc!\\>"
  "       {[stuff]}\\_visible alcohole will be\\confiscated. toxicated\\people will be sent"
  "\\on a joggetur.\\rasism and personal\\injuries will not be\\tolerated\\>"
  "       {[contact]}\\_web page:\\www.neutralzone.org -\\/romantica\\_mail:\\ hermanse@online.no"
  "\\_see you there!\\";
/* Kontrolltegn: '>' = neste side, '\\' = linjeskift (17 px), '_' = halvt linjeskift (8 px),
   '[' / ']' = stor skrift av/på (glyph 3 px høyere), ' ' = mellomrom (11 px).            */

/* ---------- Hjelperutiner ---------- */

void *Alloc(int size)             /* 0x0BD2: DOS32 int 31h, AX=EE42h */
{
    void *p = dos32_alloc(size);
    if (!p) { puts("Out of memory"); exit(); }
    return p;
}

void WaitVRetrace(void)           /* 0x09ED */
{
    while (inp(0x3DA) & 8);
    while (!(inp(0x3DA) & 8));
}

void FlushKeyboard(void)          /* setter BIOS tastaturbuffer-hode = hale (0x41C = 0x41A) */
{
    *(short *)0x41C = *(short *)0x41A;
}

byte *PalPtr;
int   PcxWidth, PcxHeight;
void LoadPCX(char *name, byte *dest)   /* 0x0EB3 */
{
    byte *buf = PcxBuf;                       /* 192 KB midlertidig buffer */
    int size = read_whole_file(name, buf);
    PcxWidth  = *(short *)(buf + 8)  + 1;     /* xmax+1 */
    PcxHeight = *(short *)(buf + 10) + 1;     /* ymax+1 */
    byte *src = buf + 128;
    for (int n = 0; n <= PcxWidth * PcxHeight; ) {   /* PCX RLE */
        byte b = *src++;
        if ((b & 0xC0) == 0xC0) { int c = b & 0x3F; memset(dest, *src++, c); dest += c; n += c; }
        else                    { *dest++ = b; n++; }
    }
    byte *p = buf + size - 0x301;
    while (*p++ != 0x0C);                     /* finn 256-farge palett-markøren */
    PalPtr = p;
    for (int i = 0; i < 256; i++) SetDAC(i, p[i*3] >> 2, p[i*3+1] >> 2, p[i*3+2] >> 2);
}

void CopyPalette(byte *dst)       /* 0x13D9 */
{
    for (int i = 0; i < 765; i++) dst[i] = PalPtr[i] >> 2;
    memset(MainPal, 0, 9);        /* farge 0-2 = svart (alltid MainPal, også ved IntroPal) */
}

void SetFadedPalette(byte *pal)   /* 0x152A */
{
    for (int i = 0; i < 256; i++) {
        byte r = min(63, pal[i*3]   * Fade >> 8);
        byte g = min(63, pal[i*3+1] * Fade >> 8);
        byte b = min(63, pal[i*3+2] * Fade >> 8);
        SetDAC(i, r, g, b);
    }
}

void BuildSinTables(void)
{
    for (int i = 0; i <= 0x200; i++) {
        SinTab[i] = fsin((i * 4) / 326.0) * SinAmp;            /* 4/326 ≈ 2π/512 */
        CosTab[i] = fsin(((i + 128) * 4) / 326.0) * SinAmp;
    }
}

void BuildTunnelTable(void)
{
    byte *t = TunnelTab;
    for (int row = 200; row > 0; row--)
        for (int col = 256; col > 0; col--) {
            int d = col + 32;
            *t++ = (-(row - 100) * 128) / d + 0x7F;   /* u */
            *t++ = 0x2400 / d + 0xE0;                 /* v (dybde) */
        }
}

void BuildShadeTable(void)
{
    memset(ShadeTab, 0, 256);
    for (int k = 0; k < 192; k++) ShadeTab[64 + k] = k / 16;
}

/* ---------- Tekst ---------- */

void RenderText(void)             /* 0x11D8: tegner hele scrolleteksten inn i TextPages */
{
    byte *page = TextPages;
    int bold = 0;
    for (char *s = ScrollText; *s; s++) {
        byte *dst = page + TextY * 1600 + TextX;
        char c = *s;
        if (c == '>')  { page += 320; TextX = 8; TextY = 35; continue; }
        if (c == '[')  { bold = 3; continue; }
        if (c == ']')  { bold = 0; continue; }
        if (c == '_')  { TextY += 8;  TextX = 0; c = 0; }      /* NB: faller videre som mellomrom */
        if (c == '\\') { TextY += 17; TextX = 0; c = 0; }
        if (c > ' ') {
            int idx = strchr(Charset, c) - Charset;
            byte *g = FontGfx + FontWidth + idx * 14 + 2;     /* rad 1, kolonne idx*14+1 */
            for (int y = 0; y < 16 + bold; y++, dst += 1600, g += FontWidth + 1)
                for (int x = 0; x < 13; x++)
                    if (g[x]) dst[x] = g[x];
        }
        TextX += 11;
    }
}

/* ---------- Effekter, del 1 (intro) ---------- */

void RotoZoom(void)               /* 0x112B: "vinglende" rotozoomer over IntroTex */
{
    int s  = SinTab[Frame & 511];
    int c  = CosTab[(Frame + (s >> 4)) & 511];
    int du = s * 8, dv = c * 8;
    int u0 = c << 5, v0 = s << 5;
    byte *d = VScreen;
    for (int y = 0; y < 200; y++) {
        int u = u0, v = v0;
        for (int x = 0; x < 320; x++) {
            byte tx = v >> 16;
            byte ty = (v >> 24) + (u >> 16);
            *d++ = IntroTex[ty * 256 + tx];
            u += du; v -= dv;
        }
        u0 += dv; v0 += du;
    }
}

void DrawLogoMask(void)           /* 0x15A8: stanser ut ROMANTICA'98 i svart, bokstav for bokstav */
{
    byte *s = LogoMask, *d = VScreen + 84 * 320 + 22;
    for (int y = 0; y < 32; y++, s += 276, d += 320)
        for (int x = 0; x < LogoWidth; x++)
            if (s[x] == 1) d[x] = 0;
}

/* ---------- Effekter, del 2 (invitasjonen) ---------- */

void DrawTunnel(void)             /* 0x1409 */
{
    byte *t = TunnelTab, *d = VScreen;
    for (int y = 0; y < 200; y++, d += 64)
        for (int x = 0; x < 128; x++, t += 4, d += 2) {
            byte u = t[0], v = t[1] + Frame;
            byte c  = EarthTex[v * 256 + u];
            int  lo = (c & 15) - ShadeTab[x * 2];
            if (lo < 0) lo = 0;
            d[0] = d[1] = (c & 0xF0) + lo;     /* 16 nyanser per fargerad */
        }
}

void DrawBanner(void)             /* 0x13A6: rad 1-38, farge 0 er gjennomsiktig */
{
    for (int i = 320; i < 320 + 38 * 320; i++)
        if (BannerGfx[i]) VScreen[i] = BannerGfx[i];
}

void DrawTextPage(void)           /* 0x1483 */
{
    byte *s = TextPages + 35 * 1600 + ScrollX * 2;
    byte *d = VScreen + 45 * 320;
    for (int y = 0; y < 144; y++, s += 1600, d += 320)
        for (int x = 0; x < 320; x++)
            if (x >= 256 || s[x] != 1) d[x] = s[x];   /* farge 1 = gjennomsiktig, ikke helt til høyre */
}

void DrawFlower(void)             /* 0x135F: blomsterfigur nede til høyre */
{
    byte *s = FlowerGfx + FlowerFrame * 90, *d = VScreen + 115 * 320 + 232;
    for (int y = 0; y < 85; y++, s += 450, d += 320)
        for (int x = 0; x < 88; x++)
            if (s[x]) d[x] = s[x];
}

/* ---------- Hovedprogram (entry 0x15F6) ---------- */

void main(void)
{
    FontGfx   = Alloc(0x5208);   FlowerGfx = Alloc(0x9E34);
    VScreen   = Alloc(64000);    EarthTex  = Alloc(90000);
    BannerGfx = Alloc(0x11800);  TunnelTab = Alloc(0x20000);
    Alloc(0x10000);              /* tildeles, men brukes aldri */
    TextPages = Alloc(5 * 64000);
    LogoMask  = Alloc(9600);     PcxBuf    = Alloc(0x2FFFF);

    USS_Init();
    if (USS_Setup() == FAIL) goto bye;              /* lydkort-menyen til Useless Sound System */
    Module = XM_Load(".\\data\\HIGH_HPS.xm");
    if (!Module) goto bye;

    SetVideoMode(0x13);                             /* 320x200x256 */
    LoadPCX(".\\data\\font26.PCX",   FontGfx);   FontWidth = PcxWidth;
    LoadPCX(".\\data\\banner2.pcx",  BannerGfx);
    LoadPCX(".\\data\\blomster.pcx", FlowerGfx);
    LoadPCX(".\\data\\earth2.pcx",   EarthTex);  CopyPalette(MainPal);
    LoadPCX(".\\data\\st_bak.pcx",   IntroTex);  CopyPalette(IntroPal);
    LoadPCX(".\\data\\romintro.pcx", LogoMask);

    memset(VScreen, 1, 64000);
    memset(TextPages, 1, 5 * 64000);                /* 1 = gjennomsiktig */
    BuildTunnelTable();
    BuildShadeTable();
    TextX = 8; TextY = 35;
    RenderText();

    USS_Play(Module, 0);
    for (int i = 0; i < 20; i++) WaitVRetrace();
    BuildSinTables();

    /* --- Del 1: rotozoomer + logo som tegnes bokstav for bokstav --- */
    Fade = 5; Frame = 0;
    do {
        RotoZoom();
        DrawLogoMask();
        WaitVRetrace();
        SetFadedPalette(IntroPal);
        memcpy((void *)0xA0000, VScreen, 64000);
        FlushKeyboard();
        Frame++;

        if (Fade >= 240 && Frame >= 128) {
            int i = ((Frame - 127) >> 4) + 1;
            if (i >= 17) EscTimer++;                /* ferdig -> fade ut */
            if (i <= 11) LogoWidth = LogoWidths[i];
        }
        if (inp(0x60) == 1) EscTimer++;             /* ESC hopper over */
        if (EscTimer >= 1)      Fade -= 2;
        else if (Fade < 254)    Fade += 2;
    } while (Fade >= 5);

    EscTimer = 0; Fade = 0;
    SetFadedPalette(IntroPal);
    memset(VScreen, 0, 64000);
    for (int i = 0; i < 60; i++) WaitVRetrace();
    FlushKeyboard(); FlushKeyboard();

    /* --- Del 2: invitasjonen --- */
    Fade = 2; ScrollTarget = 0; Frame = 0;
    do {
        DrawTunnel();
        DrawBanner();
        DrawTextPage();
        DrawFlower();
        if (QuitCount == 0 && Fade < 255) Fade++;
        WaitVRetrace();
        SetFadedPalette(MainPal);
        memcpy((void *)0xA0000, VScreen, 64000);

        if (EscTimer   && --EscTimer   == 0) FlowerFrame = 4;
        if (LeftTimer  && --LeftTimer  == 0) FlowerFrame = 4;
        if (RightTimer && --RightTimer == 0) FlowerFrame = 4;

        if (ScrollX < ScrollTarget)      ScrollX += 8;
        else if (ScrollX > ScrollTarget) ScrollX -= 8;
        FlushKeyboard();
        Frame++;

        byte key = inp(0x60);
        if (key == 0x01) {                          /* ESC */
            FlowerFrame = 0; EscTimer = 25; QuitCount++;
        } else if (key == 0x4B && !LeftTimer) {     /* pil venstre */
            FlowerFrame = 1; LeftTimer = 25;
            ScrollTarget -= 160;
            if (ScrollTarget < 0) ScrollTarget = 4 * 160;
        } else if (key == 0x4D && !RightTimer) {    /* pil høyre */
            FlowerFrame = 2; RightTimer = 25;
            ScrollTarget += 160;
            if (ScrollTarget > 4 * 160) ScrollTarget = 0;
        }
        if (QuitCount > 1) Fade--;
        if (QuitCount > 2) Fade--;
    } while (Fade >= 2);

    SetVideoMode(3);
    print("det var egentlig alt det... bare du scrolla!\n\r");
    print("\n\r");
    USS_Stop(Module);
bye:
    print(" a TP-multimedia produksjon tm\n\r");
    print("\n\r");
    exit(0);
}

/*
 * Rutiner som finnes i binærfila, men som aldri kalles (rester fra et eget "bibliotek"):
 *   0x083C  4x4-blokk interpolert teksturtegner
 *   0x09D9  sinusoppslag
 *   0x09FC  3D-rotasjon av punkt rundt tre akser (vinkler på 0x806/0x808/0x80A)
 *   0x0BDF  laster en 64x64 høydekart-fil og interpolerer den til et gitter
 *   0x0E28  VESA hicolor: konverterer 24-bit -> 16-bit 5:6:5 og skriver med bankbytte (4F05h)
 *   0x0FF6  generell "les hel fil til minne"
 *   0x1055  blur/glatting (gjennomsnitt av 8 naboer) på en 768 px bred buffer
 * I tillegg: filnavnet ".\data\dato.pcx" og teksten "using USM Play v1.0a." brukes ikke,
 * og feilmeldingen "USM-player returned a unkown error" kan aldri nås.
 */
