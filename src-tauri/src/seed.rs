use rand::seq::SliceRandom;
use sha2::{Digest, Sha256};
use std::sync::Mutex;

/// Word list para generación de mnemonic (256 palabras BIP39 simplificadas)
const WORD_LIST: &[&str] = &[
    "abaco", "abrir", "acero", "acida", "actas", "acuna", "adapt", "adega", "adios", "adobe",
    "aduan", "aerea", "afect", "agata", "agile", "agora", "agude", "ahijado", "aimara", "ajena",
    "akira", "alamos", "alba", "alcal", "aldab", "aleman", "alfab", "algas", "alico", "alima",
    "aliso", "almac", "almir", "aloha", "alpes", "altas", "altur", "alumb", "alvar", "amada",
    "ambar", "amena", "amiga", "amino", "amist", "ampla", "amule", "ancla", "andar", "aneco",
    "ancla", "anima", "anito", "anual", "anzol", "apare", "apero", "apice", "aplan", "apost",
    "apret", "aquae", "arana", "arbol", "arces", "archi", "ardor", "areal", "arete", "argos",
    "arias", "arida", "arido", "armas", "armer", "aroma", "arpon", "arroy", "artes", "artus",
    "aruda", "arzob", "asado", "asilo", "asman", "asper", "astil", "atada", "ataud", "atomo",
    "atras", "atune", "aureo", "avalo", "avion", "axila", "ayuda", "azada", "azale", "aznar",
    "azote", "azul", "bably", "bahia", "baila", "bajar", "balas", "balde", "balet", "balla",
    "banco", "banda", "banjo", "barba", "barco", "baria", "bario", "barni", "barra", "basal",
    "basur", "batir", "baile", "bajan", "belle", "beret", "berma", "besti", "betun", "bicho",
    "bicho", "bifen", "bigas", "bille", "bimba", "bingo", "biras", "birla", "bisel", "bizca",
    "bizco", "blanc", "bledo", "bloca", "bloc", "blusa", "boato", "bobos", "bocel", "bocin",
    "bodeg", "bofet", "bolsa", "bomba", "bombo", "bonac", "bondi", "bonos", "borda", "boreo",
    "borla", "borni", "borra", "bosar", "botan", "botar", "botin", "boton", "boxeo", "boyal",
    "bravo", "braza", "brazo", "breas", "brete", "brida", "brido", "brill", "brisa", "broca",
    "brota", "bruma", "bruno", "bruto", "buche", "bucle", "buffo", "buho", "buitr", "bular",
    "bulev", "bulla", "bulto", "buril", "burra", "busto", "buten", "buzon", "cabal", "caber",
    "cabra", "cacao", "cacao", "cacts", "cadiz", "caeri", "cafre", "caigo", "cajas", "calas",
    "calca", "caldo", "calef", "calif", "calla", "calle", "calma", "calor", "calva", "calvo",
    "cama", "cambi", "camio", "campi", "canal", "cance", "candi", "canon", "canto", "capas",
    "capaz", "capel", "capital", "capri", "carac", "caram", "carat", "carbon", "carca", "carda",
    "cardo", "cargo", "cariz", "carme", "carni", "carpa", "carpi", "carpo", "carro", "carta",
    "casco", "casos", "casta", "catar", "catre", "cauda", "cauta", "cauto", "cavia", "cazar",
    "cedro", "cegar", "ceja", "celar", "celes", "celia", "celta", "cenit", "censo", "centa",
    "cepar", "cepos", "ceras", "cerca", "cerco", "cerdo", "cerea", "ceros", "certa", "cesar",
    "cesio", "cesta", "chale", "chama", "chapa", "chica", "chico", "chile", "china", "chisa",
    "choco", "chofe", "chola", "cholo", "chopo", "choza", "chulo", "chupa", "churl", "churu",
    "ciber", "cicla", "cidra", "ciena", "cielo", "cifra", "cigar", "cilla", "cimar", "cimer",
    "cinta", "cinto", "cipos", "circe", "circo", "cisco", "cisne", "citar", "civil", "claim",
    "clama", "clapa", "claro", "clase", "clava", "clema", "clima", "clipe", "cloca", "clota",
    "clubs", "coati", "cobij", "cobra", "cobro", "coche", "cocin", "cocoa", "colas", "colin",
    "colla", "colme", "colmo", "colon", "color", "colpa", "colza", "comba", "combi", "comer",
    "comet", "comio", "compa", "compu", "comun", "conca", "conde", "confi", "conga", "congo",
    "conil", "consu", "conta", "contar", "copal", "copla", "copta", "coral", "corar", "corca",
    "corda", "coria", "cormo", "corni", "coroa", "corpo", "corra", "corro", "corsa", "corte",
    "corva", "coser", "cosmo", "costa", "costr", "cotan", "cotil", "cowboy", "crane", "crece",
    "credo", "crema", "crepa", "crepo", "cruda", "crudo", "cruel", "cruza", "cubos", "cucci",
    "cuche", "cucos", "cudad", "cuego", "cufar", "culan", "culat", "culeb", "cumba", "cumbo",
    "cumpl", "cunar", "cundi", "cunet", "cuota", "cupon", "cural", "curar", "curio", "curro",
    "curso", "curva", "cuyas", "cuyos", "dable", "daces", "dadar", "dagas", "dahae", "daimi",
    "dajia", "dalar", "dalti", "damos", "danas", "danza", "dapas", "daqui", "daria", "datar",
    "datil", "dator", "deban", "debar", "deber", "deces", "decir", "dedal", "dedos", "deform",
    "dehes", "dejar", "delas", "delia", "delir", "delta", "deman", "demas", "dende", "denia",
    "densa", "denso", "depar", "depend", "depor", "derbi", "derma", "dermo", "desde", "desga",
    "desma", "despe", "deste", "detal", "detar", "detox", "deuda", "diabl", "diafr", "diant",
    "diari", "dicas", "diedo", "diez", "difus", "digan", "digest", "dijes", "dilat", "dildo",
    "dilem", "diluv", "dimer", "dimin", "diosa", "disci", "disco", "diseñ", "dislo", "dispa",
    "dispersion", "ditas", "diurn", "divan", "divas", "diver", "divin", "divis", "dixit",
    "dizque", "docen", "docta", "docto", "dolar", "dolce", "dolor", "domar", "domen", "domin",
    "donar", "donat", "donde", "donde", "donju", "donna", "dopar", "doral", "dorsa", "dotal",
    "douar", "draga", "drama", "drené", "driza", "dromo", "dropa", "ducha", "duelo", "duero",
    "duler", "dumbo", "dunar", "dunar", "dupla", "duras", "durez", "duros", "duras", "ebano",
    "ebrio", "echai", "echas", "echin", "edema", "edila", "edilo", "edipo", "edita", "editor",
    "edran", "edran", "educa", "efebo", "efeto", "efuso", "egena", "egeno", "egida", "eguar",
    "ejido", "ejion", "ejote", "elata", "elato", "elema", "elena", "eleve", "elfos", "eliga",
    "elige", "elijo", "ellos", "elote", "eluda", "elude", "eluso", "email", "emano", "embar",
    "emerg", "emilo", "emiro", "empec", "emuls", "enano", "encar", "encio", "endos", "enedi",
    "eneida", "enema", "enero", "enfad", "enfié", "enfin", "engar", "englo", "enojo", "enria",
    "ensay", "enter", "entra", "entro", "envas", "envio", "enzim", "eolos", "epica", "epico",
    "epoca", "equip", "eraño", "erario", "eriza", "erizo", "ermit", "erran", "errar", "error",
    "eruga", "esbio", "escab", "escap", "escaq", "escar", "escaz", "esfer", "eskin", "esmir",
    "espec", "espes", "espia", "espin", "espio", "espoj", "espos", "esqui", "esrit", "estab",
    "estad", "estan", "estar", "estel", "estep", "ester", "estig", "estir", "estof", "estol",
    "estor", "estra", "estre", "estri", "estro", "estuf", "estur", "esval", "etalon", "etano",
    "etapa", "etico", "etilo", "etnea", "etneo", "etnico", "etusa", "eucal", "evada", "evado",
    "eveli", "evita", "evohe", "evuco", "exces", "exija", "exijo", "exigi", "eximo", "exisa",
    "exise", "exito", "exodi", "expia", "expio", "expon", "expos", "exsig", "extas", "extra",
    "fable", "facaz", "facha", "facia", "facto", "fadar", "fagot", "faena", "fagos", "fajas",
    "fajin", "falar", "fallo", "falso", "falta", "famas", "fango", "faena", "faeno", "fajor",
    "falaz", "falle", "fango", "faras", "fario", "farol", "farsa", "fasea", "fasol", "faste",
    "fatal", "fatua", "fatuo", "fauna", "favel", "favor", "faxea", "feble", "fecho", "fefas",
    "feide", "feír", "feliz", "felpa", "felpe", "femen", "femur", "fenol", "feral", "feria",
    "feroz", "ferra", "ferry", "ferva", "feste", "fetal", "fetos", "fiado", "fianc", "fiaos",
    "fiaro", "ficas", "fices", "ficha", "fideo", "fieis", "fiele", "fielt", "fiero", "fiest",
    "figle", "filan", "filas", "filete", "filia", "filip", "filmé", "filmo", "filon", "filos",
    "filtr", "fimbri", "final", "finca", "finde", "finge", "finio", "finis", "finta", "fiole",
    "firme", "fisco", "fisga", "fisio", "fison", "fista", "fiste", "flacc", "flaco", "flama",
    "flame", "flano", "flash", "flato", "flavo", "fleco", "fleja", "flema", "fleme", "fleta",
    "flexo", "flipa", "flipo", "floja", "flojo", "flora", "flore", "flota", "flote", "fludo",
    "fluir", "fluis", "flume", "flora", "flote", "fluct", "fluir", "fluid", "fluma", "flume",
    "fluxo", "fobia", "focal", "focha", "fogar", "fogue", "foisa", "foiso", "folga", "folia",
    "folio", "folks", "fonda", "fondo", "fonil", "fonos", "foral", "forca", "foren", "forja",
    "forma", "forro", "forte", "forzar", "fosca", "fosfo", "foso", "fotó", "foyer", "fracc",
    "fraga", "fraja", "franc", "frane", "frase", "fraud", "freia", "freir", "frena", "freno",
    "fresa", "fresc", "freta", "freyr", "friga", "friki", "friso", "frita", "frito", "frivol",
    "froma", "front", "frota", "frufr", "frugal", "fruta", "fruto", "fucia", "fuego", "fuero",
    "fuert", "fuesa", "fugaz", "fugue", "fulan", "fulla", "fumad", "fumas", "fumos", "funar",
    "funci", "fungo", "furia", "furor", "fusca", "fusco", "fusil", "fusta", "fuste", "futur",
    "gabar", "gabba", "gabin", "gable", "gacel", "gacha", "gadit", "gafas", "galan", "galas",
    "galgo", "galoa", "galon", "galop", "gamas", "gambé", "gambé", "gamed", "gamil", "gamio",
    "gamo", "ganch", "ganet", "gansa", "ganso", "garab", "garau", "garba", "garbo", "garza",
    "gasa", "gasto", "gatón", "gauca", "gauda", "gauza", "gavet", "gayar", "gazap", "gazmo",
    "geek", "gelat", "gemma", "gemir", "gemma", "genet", "genio", "gente", "geoda", "gerbo",
    "germi", "gesta", "geste", "gigle", "giglo", "gijon", "gimio", "ginec", "girei", "giros",
    "gitán", "glaci", "glad", "gland", "glasa", "glean", "gleba", "glide", "glima", "globo",
    "gloom", "glops", "glori", "glosa", "gluma", "gluon", "gnomo", "gobbi", "gobio", "godon",
    "goles", "golfa", "golfo", "golpe", "gomer", "gomia", "gonce", "gonda", "gonet", "gonfi",
    "gorja", "gorma", "gorri", "gorro", "gotha", "gotic", "gouda", "gauva", "goyal", "gozne",
    "gozos", "graba", "graci", "grada", "grado", "grafe", "grafs", "graha", "grail", "graja",
    "grana", "grand", "grane", "grano", "grapa", "grasa", "grata", "grato", "grava", "grazn",
    "greca", "greco", "gredo", "grefa", "grege", "grego", "greix", "gresa", "greva", "grifa",
    "grife", "grill", "grima", "gripe", "gripi", "grisa", "griso", "grita", "grito", "grogs",
    "groix", "groma", "groom", "grota", "grove", "grúa", "grufe", "gruñi", "gruta", "guabo",
    "guaco", "guado", "guaja", "guano", "guape", "guapo", "guara", "guard", "guare", "guari",
    "guarn", "guasa", "guaso", "guata", "guato", "guaxe", "guayo", "gueld", "guero", "guerra",
    "guiad", "guial", "guiar", "guila", "guilo", "guina", "guino", "guion", "guise", "guita",
    "guiño", "gular", "gulem", "gumia", "gurda", "gusto", "gutur", "guzla", "habar", "haber",
    "había", "habit", "habiz", "habla", "hablo", "hables", "habon", "hacer", "hacha", "hacho",
    "hadiz", "hafiz", "hagas", "halal", "halco", "halda", "halle", "hallo", "hamal", "hamen",
    "hampa", "hampo", "hanzo", "hapax", "haplo", "harán", "haras", "haré", "harén", "harés",
    "haría", "harón", "harta", "harto", "hasel", "hasta", "hataco", "hateo", "hátiz", "hato",
    "haute", "havar", "havito", "hayal", "hayas", "hebra", "hecho", "helar", "helio", "hemen",
    "henar", "henil", "henos", "hepar", "herba", "herbo", "hered", "heres", "heria", "herir",
    "herma", "heroe", "hespe", "heteo", "hetra", "hexen", "hiato", "hiber", "hidra", "hijos",
    "hilar", "hiles", "hilos", "himen", "himno", "hinas", "hinge", "hiper", "hipic", "hipno",
    "hiram", "hirco", "hirio", "hirvd", "hisca", "hisop", "hobby", "hoceo", "hocio", "hojas",
    "holar", "holes", "holga", "holgo", "hollo", "homar", "homer", "homun", "honar", "honco",
    "hondo", "hopas", "hopeo", "horca", "horda", "horma", "horno", "horra", "horro", "hosco",
    "hosta", "hotel", "hotre", "house", "hovit", "huaca", "huaco", "huair", "huasa", "huaso",
    "huate", "huato", "hubo", "hucha", "hucho", "hucia", "huien", "huios", "hule", "hulla",
    "hullo", "humar", "humor", "hurac", "hurar", "hurra", "hurst", "husma", "husme", "hutia",
    "iambos", "icaco", "iceis", "ichth", "icono", "ictus", "ideal", "idear", "idilio", "idolo",
    "idone", "iglú", "ignea", "igneo", "ignis", "igual", "iguar", "ileal", "ileso", "ilium",
    "ilusa", "iluso", "image", "imago", "imane", "imán", "impar", "impel", "impio", "impon",
    "impul", "inane", "incan", "incas", "incoa", "incog", "incur", "india", "indio", "inepto",
    "inerm", "inert", "infan", "infer", "infes", "infla", "infra", "ingen", "ingre", "inici",
    "inico", "injer", "innato", "inspe", "insta", "inter", "intus", "inven", "invin", "invoc",
    "iondo", "iracu", "irade", "irani", "irás", "iride", "irido", "iría", "irisa", "iriso",
    "ironía", "irrui", "irrup", "isabel", "isdra", "islad", "islar", "islot", "isleo", "ismos",
    "istmo", "italo", "itera", "itere", "itice", "ivica", "ivori", "izaje", "izanó", "izmir",
    "izote", "izqui", "jabal", "jabato", "jable", "jabora", "jacak", "jadeo", "jadeo", "jaguar",
    "jailos", "jaime", "jalar", "jalón", "jamaa", "jamás", "jameo", "janeo", "japón", "jaque",
    "jaral", "jareta", "jarro", "jaspe", "jatib", "jauja", "jaula", "jauri", "jayan", "jazmín",
    "jazz", "jeito", "jemal", "jenar", "jeneu", "jerez", "jerpe", "jesús", "jetar", "jeto",
    "jigar", "jigote", "jimio", "jiñar", "jiote", "jirel", "jiso", "jitsu", "jobar", "jockey",
    "joder", "jofor", "jolín", "jondo", "jonio", "jopar", "jopeo", "jorco", "jordi", "jorfe",
    "jorga", "jorge", "jorná", "jorrar", "josear", "josín", "joule", "joven", "joyel", "joyero",
    "juana", "jubón", "jucar", "juche", "juega", "juego", "juerga", "jufre", "julia", "julio",
    "jumel", "junto", "jopia", "jurar", "jurel", "juseo", "justa", "justo", "jutia", "juvia",
    "juzga", "kabila", "kabum", "kafir", "kakis", "kamas", "kanes", "kanji", "karst", "kayak",
    "kebab", "kebla", "keels", "kefir", "kelme", "kelps", "kenaf", "kenia", "kepis", "kerné",
    "keter", "khadi", "khmer", "kiack", "kicap", "kieve", "kilot", "kimono", "kinde", "kines",
    "kinpe", "kinto", "kiote", "kiosco", "kirio", "kisan", "kitty", "kiwi", "klaxon", "klebs",
    "kneipp", "knock", "koban", "koine", "kolhoz", "kolmio", "kompa", "konak", "kondu",
    "konko", "kontu", "kopec", "kopel", "kopek", "koran", "korat", "korea", "korma", "korse",
    "koshi", "kraft", "kraus", "kriol", "krona", "krone", "kruki", "kudzu", "kulak", "kumys",
    "kurda", "kurdo", "kursaal", "kvas", "kyma", "kyrie", "labar", "lábar", "labia", "labil",
    "labio", "labor", "labra", "lacar", "lacer", "lacha", "lacio", "lacro", "lacta", "ladar",
    "ladero", "ladra", "lagar", "lago", "lagos", "laica", "laico", "lajar", "lajea", "lalar",
    "lalo", "llama", "llana", "llano", "llave", "lleco", "llega", "llema", "llena", "lleno",
    "llera", "lleud", "lleva", "llibl", "llico", "llist", "lloca", "lloco", "llora", "lloro",
    "llosa", "lluvi", "locas", "locha", "locos", "locro", "locum", "lodeo", "lodón", "logia",
    "logro", "loica", "loido", "loina", "loipe", "loker", "lolaje", "lollo", "lomas", "lomos",
    "lonco", "longa", "longo", "lonja", "loor", "loque", "lorca", "loren", "lorza", "losas",
    "lotar", "loteo", "lotos", "loxa", "lucas", "lucen", "lucia", "lucir", "lucro", "ludio",
    "ludir", "luego", "lueñe", "lugre", "luid", "lulio", "lumbo", "lumin", "lumpen", "lunar",
    "lunch", "lundu", "lunes", "lunfa", "luneta", "luper", "luria", "lustra", "lustre", "lutea",
    "luteo", "lutar", "luter", "lutto", "luvia", "luxar", "luzca", "lycra", "macal", "macar",
    "macer", "macho", "macia", "macis", "macro", "macul", "madre", "mafia", "magar", "magia",
    "magio", "magister", "magma", "magna", "magni", "magno", "magra", "magro", "mahon", "maida",
    "maíz", "major", "malal", "malas", "malca", "malco", "malee", "malga", "mallo", "malo",
    "malos", "malta", "malva", "mamba", "mambo", "mamei", "mamón", "mamut", "manal", "manar",
    "manca", "manco", "manda", "mandi", "manea", "manel", "manes", "manga", "mango", "mani",
    "manir", "manos", "mansa", "manso", "manta", "mante", "mantó", "manus", "maoma", "maori",
    "mapas", "mapeo", "maple", "maque", "maqui", "marav", "marca", "marco", "mares", "marga",
    "marlo", "marmo", "marqu", "marra", "marro", "marsa", "marta", "marte", "marzo", "masas",
    "mascó", "maser", "masia", "maslo", "mason", "masos", "masto", "matas", "matea", "mateo",
    "matiz", "mator", "matte", "maula", "maura", "mauro", "mayer", "mayor", "mazar", "mazá",
    "mazo", "mazur", "meada", "meado", "mear", "meato", "mecen", "mecha", "medas", "medio",
    "medra", "megano", "megio", "meigo", "meigo", "meita", "mejun", "melar", "melco", "melee",
    "melga", "melgo", "melic", "melis", "mella", "melle", "melló", "meloa", "melon", "melos",
    "membr", "memes", "memor", "menar", "mende", "meneo", "mengu", "menor", "menos", "menos",
    "mensu", "menta", "mento", "menué", "meoqui", "merca", "merco", "merlo", "merma", "meros",
    "mesar", "meses", "mesmo", "metaf", "metas", "meteo", "meter", "metra", "metros", "mezcl",
    "miago", "miale", "miant", "miasm", "mibal", "micas", "micel", "micho", "micon", "micra",
    "micro", "midas", "midri", "miel", "mielo", "migar", "migas", "migra", "milan", "milde",
    "miled", "miles", "milla", "millo", "millor", "mimar", "mimbr", "mimia", "mimo", "minar",
    "minca", "minci", "minio", "minor", "mioma", "mirla", "mirlo", "miron", "mirra", "mirto",
    "misal", "misia", "misio", "mismo", "misto", "mitad", "mitos", "mitra", "mizar", "mizzo",
    "moble", "mocar", "mocas", "moceo", "mocha", "mochi", "mochil", "mocito", "modco", "modal",
    "modos", "moers", "mofle", "mogol", "mogon", "moice", "moine", "moisés", "mojil", "mojín",
    "molar", "molda", "mole", "molet", "molle", "momia", "monas", "monco", "monde", "mondo",
    "mongo", "moni", "mono", "monos", "monta", "monté", "monto", "monza", "moque", "morad",
    "moral", "morar", "morat", "morba", "morce", "morde", "morel", "moren", "mores", "morfó",
    "morfo", "morga", "moria", "morín", "morla", "morro", "morsa", "mortal", "moruca", "morvo",
    "mosca", "mosco", "moses", "mosto", "mostr", "motel", "motin", "motor", "moton", "motos",
    "moule", "mover", "movie", "moxar", "moxón", "moyas", "moza", "mozo", "muare", "mucha",
    "muche", "mucos", "mudar", "mudra", "mueca", "muela", "muera", "muero", "mueso", "muevo",
    "mufla", "mufti", "mugre", "mujer", "mular", "muleo", "muleta", "mulo", "mulos", "mumbo",
    "mumbo", "munge", "mungir", "munta", "muñir", "mural", "murar", "murci", "mures", "muro",
    "murra", "murri", "musar", "musca", "musco", "museo", "musha", "musit", "muslo", "muste",
    "mutar", "mutil", "mutis", "muzar", "nacer", "nacho", "nadal", "nadie", "nadir", "naipe",
    "najar", "najor", "nalda", "nalga", "namor", "nance", "nancy", "nando", "nanos", "naper",
    "napia", "narco", "nardo", "nariz", "narru", "nasal", "nason", "natío", "natura", "nause",
    "naval", "navar", "naves", "navio", "nazca", "nebla", "nebon", "negar", "negra", "negro",
    "negus", "neies", "neldo", "némesis", "nene", "nenei", "nengo", "nenia", "neper", "neret",
    "nervi", "nesga", "netas", "neteo", "netos", "neura", "neuro", "nevad", "nevar", "nevas",
    "nevazo", "nexos", "niara", "nibaba", "nibiru", "niche", "nicho", "nicro", "niebla", "nieta",
    "nieto", "nieve", "nigra", "nigre", "nigua", "nilon", "nimba", "nimbo", "ninfo", "ninot",
    "nipas", "niple", "nipón", "nique", "nisca", "niso", "nitro", "nivel", "noble", "nóbita",
    "nobla", "noche", "noced", "nocir", "nodos", "nodo", "noema", "noeno", "nofre", "nolit",
    "nomad", "nomas", "nombre", "nombr", "nonas", "nones", "nopal", "noque", "norma", "norse",
    "nostoc", "notar", "notas", "notici", "notro", "nouga", "noven", "novio", "nuca", "nudo",
    "nudus", "nueva", "nueve", "nuevo", "numen", "numos", "nunca", "nuria", "nutre", "nutri",
    "oasis", "obelo", "obice", "obito", "oblea", "obrep", "ocaso", "occid", "occis", "ocelo",
    "ochav", "ochoa", "ocian", "ocult", "ocumo", "ocurrir", "odiar", "odio", "odor", "ofert",
    "oficio", "ofita", "ogro", "ohmio", "oír", "ojal", "ojete", "ojival", "ojos", "okapi",
    "olaje", "olear", "oleen", "oleno", "oleos", "olera", "olero", "olibán", "oligo", "oliva",
    "olivo", "ollao", "ollar", "olmed", "ombligo", "ominar", "omiso", "omita", "omito", "omnes",
    "once", "ondas", "ondeo", "ondul", "onza", "opaco", "opera", "opime", "opinar", "opone",
    "optar", "optas", "óptica", "opuse", "oración", "oral", "orar", "orbe", "orbital", "orcas",
    "orco", "ordal", "ordas", "orden", "orear", "oreja", "orfre", "orgas", "orgia", "oriar",
    "orice", "orina", "orino", "oris", "orla", "orlar", "ornar", "ornis", "orria", "orsai",
    "oruga", "orugo", "orval", "osadía", "osado", "oscar", "oseno", "osis", "osito", "osmio",
    "ostia", "otero", "otilo", "otina", "otino", "otoba", "otorg", "otra", "otro", "ouija",
    "oxear", "oxido", "oxón", "oxte", "oye", "oyos", "ozono", "pabla", "pablo", "pacay",
    "pace", "pacer", "pacha", "pacte", "padre", "pafia", "pagar", "pagel", "pagro", "pague",
    "paico", "paila", "paine", "paira", "paise", "pajes", "palas", "palau", "palco", "palea",
    "paleo", "palio", "palma", "palme", "palmo", "palo", "palos", "palpa", "palpe", "palpo",
    "pampa", "panal", "panca", "pancho", "pandi", "panel", "panes", "pansa", "pantar", "pante",
    "papar", "papel", "papión", "papir", "papos", "paque", "parar", "parca", "parce", "parco",
    "parda", "pardo", "pared", "paria", "parir", "paros", "parpa", "parpe", "parra", "parré",
    "parte", "parto", "pasar", "pasas", "paseo", "pasmo", "paso", "paspas", "pasta", "pasto",
    "pata", "patán", "pateo", "patia", "pato", "patos", "patra", "paule", "pauli", "pausa",
    "pauta", "paved", "pavón", "pavos", "payar", "payas", "payo", "peaje", "peal", "pecar",
    "pecez", "pecha", "peche", "pedal", "pedir", "pedre", "pegar", "pegas", "pegma", "pegos",
    "peine", "peje", "pella", "pello", "pelmazo", "penal", "pene", "penes", "pengo", "penol",
    "pensa", "pense", "penso", "peón", "peor", "peper", "pepin", "peque", "peral", "perca",
    "percha", "perde", "perdi", "perej", "peret", "perla", "perno", "peros", "perse", "persa",
    "perse", "perú", "pesad", "pesar", "pesce", "pesco", "peses", "pesga", "pesgo", "pesos",
    "peste", "petar", "petas", "petau", "peteo", "petit", "petra", "petro", "petul", "peuco",
    "peumo", "peusa", "peven", "pezón", "piantar", "piar", "pica", "picar", "picaso", "picha",
    "piche", "pichel", "picho", "picia", "picho", "pico", "picor", "picos", "picra", "picul",
    "piden", "pieza", "pifano", "pifia", "pigre", "pigro", "pilar", "pilas", "pileo", "pilla",
    "pillo", "piloto", "piltra", "pimba", "pimia", "pimio", "pínam", "pinar", "pinco", "pindo",
    "pineo", "pinos", "pinta", "pinte", "pinto", "pinza", "piña", "piñar", "piñon", "piola",
    "pion", "pior", "pipa", "piper", "pipio", "pipir", "pipis", "pipón", "pique", "pira",
    "pirar", "piren", "pirlé", "piro", "pirol", "pirot", "pirri", "pirul", "pisar", "piscis",
    "pisco", "piseo", "pisos", "pista", "pisto", "pitón", "pitio", "pitir", "pito", "piton",
    "pitos", "pivote", "place", "placi", "plaga", "plago", "plana", "plano", "planta", "plata",
    "plato", "playa", "plazo", "plebe", "pleca", "plego", "plena", "pleno", "plica", "plomo",
    "pluma", "plume", "plumo", "plunk", "plusu", "pobre", "pocas", "pocka", "pocos", "podar",
    "poder", "podio", "podón", "podre", "pogue", "poisa", "poise", "polar", "polea", "polen",
    "polio", "polir", "polis", "polka", "polo", "polos", "polup", "pomar", "pombo", "pompa",
    "ponch", "poner", "ponga", "pongo", "ponle", "ponzo", "pooch", "popa", "popel", "popes",
    "popote", "poque", "porca", "porco", "porel", "porfa", "porno", "porra", "porro", "porta",
    "porto", "posar", "posca", "posco", "posda", "posde", "poseer", "posho", "posit", "posos",
    "posta", "poste", "potar", "pote", "poten", "potes", "poto", "potro", "povisa", "power",
    "pozas", "prado", "praxi", "pravo", "prear", "preco", "preda", "prefe", "prego", "premi",
    "prenda", "prepo", "preso", "previ", "prima", "primo", "prín", "prion", "prior", "prisa",
    "priva", "proas", "probe", "proca", "proco", "procur", "profir", "prois", "prole", "promé",
    "prompt", "prone", "propio", "prosa", "prostr", "proto", "prove", "provi", "prude", "pruna",
    "pruno", "púa", "puar", "pubis", "puche", "pucia", "pudín", "pudor", "pudra", "pudre",
    "pufas", "pugil", "pugna", "pugnar", "puir", "puita", "pulir", "pulpa", "pulpo", "pulse",
    "pulso", "puncha", "punco", "pungi", "punta", "punto", "pupa", "pupar", "pupil", "pupos",
    "purín", "purga", "purgo", "puria", "puro", "puron", "puspo", "putar", "putas", "putto",
    "puzle", "quark", "quaro", "quars", "quasar", "quear", "quebra", "quedé", "queer", "quela",
    "quemen", "quemi", "queno", "querer", "queso", "quia", "quias", "quiche", "quijot", "quila",
    "quila", "quilo", "quimia", "quina", "quines", "quino", "quiosco", "quiral", "quirie",
    "quiro", "quirre", "quisa", "quise", "quiso", "quita", "quite", "quizá", "quórum", "rabal",
    "rabia", "rabie", "rabil", "rabón", "racar", "racer", "racha", "racia", "raco", "radal",
    "radar", "radio", "raée", "rafal", "rafez", "rafia", "raga", "ragú", "rahú", "raíz",
    "rajada", "rajar", "rajas", "rajea", "rajen", "raji", "rala", "ralas", "ralbar", "ralea",
    "rallo", "ralo", "ramal", "ramas", "rambla", "ramio", "ramiro", "ramon", "rampón", "ramya",
    "ranas", "rance", "randa", "rapaz", "rapel", "rapero", "rapto", "raque", "rarel", "rasar",
    "rasca", "rasco", "rasel", "rases", "rasga", "rasgo", "rasgo", "raspa", "raspe", "raspo",
    "rasta", "ratear", "ratero", "ratico", "ratón", "raúco", "raudo", "raular", "rauli",
    "rauca", "rauco", "rauma", "rauta", "razón", "reala", "realo", "rebañ", "rebeca", "reble",
    "rebol", "recel", "recia", "recio", "recog", "recol", "recta", "recto", "recua", "recus",
    "redar", "redel", "redil", "redol", "redom", "reduc", "refez", "refio", "refla", "refre",
    "regad", "regai", "regal", "regir", "regla", "regleo", "regma", "regol", "regue", "rehaz",
    "rehui", "rehus", "reina", "reino", "rejal", "rejar", "rejas", "relej", "releo", "relev",
    "relin", "relox", "remar", "rematar", "rembo", "remen", "remero", "remes", "remiso",
    "remite", "remito", "remor", "remoto", "renal", "rencor", "rendir", "rene", "reneg",
    "renga", "rengo", "renil", "renir", "renos", "renta", "renuevo", "reojo", "repon",
    "reposo", "reptar", "reptil", "repun", "requé", "requi", "resal", "reses", "resio",
    "resma", "resol", "respó", "respi", "resque", "resta", "reste", "resto", "retar",
    "retazo", "retel", "retén", "reteo", "retes", "retir", "retor", "retre", "retri",
    "retró", "retro", "retaz", "reuma", "revés", "reven", "rever", "revés", "reyar",
    "rezar", "rezno", "rezos", "riada", "rial", "riata", "ribera", "ricio", "rida", "rife",
    "rifle", "rigia", "rigor", "rilea", "rilló", "rimar", "rimel", "rinde", "riñon", "riola",
    "ripar", "ripias", "risca", "risco", "risión", "rispo", "ritar", "ritma", "ritmo",
    "riñar", "robar", "rober", "robín", "robot", "roble", "roblo", "robos", "robra", "rocar",
    "roceo", "roces", "rocha", "roche", "rocié", "roció", "rock", "rodea", "rodéo", "rodeo",
    "rodias", "rodil", "rodio", "rodrí", "rogar", "rogue", "roiar", "roído", "roigo",
    "rojel", "rojizo", "rojos", "rolar", "roldo", "roleo", "rollo", "roman", "rombo",
    "romer", "romeu", "rompe", "rompo", "roncar", "ronco", "ronda", "rondó", "ronfa",
    "ronge", "ronqu", "roque", "rorar", "rorel", "rorez", "rorschach", "rosal", "rosar",
    "rosca", "rosea", "roseo", "roses", "rosjo", "rosoli", "rosti", "rotar", "roteo",
    "roten", "rotor", "rotos", "rouge", "royo", "rozar", "roznar", "ruana", "ruano",
    "rubar", "rubí", "rubio", "ruble", "rubor", "ruche", "rudas", "ruddo", "rudez",
    "rudo", "ruedas", "ruego", "ruejo", "rufas", "rufin", "rugal", "rugby", "rugir",
    "rugue", "ruina", "rujar", "ruleta", "rulfo", "rulo", "rumbo", "rumia", "rumor",
    "runga", "runos", "rupia", "ruptu", "rural", "rusco", "rusel", "rusos", "ruste",
    "ruste", "ruta", "rutar", "rutilo", "rutina", "sábado", "saber", "sabia", "sabio",
    "sable", "sabon", "sabra", "sacar", "saché", "sachi", "sacio", "sacro", "saeta",
    "safio", "sagaz", "sagra", "sagro", "sahar", "sahum", "saice", "saifa", "saino",
    "sajar", "salad", "salar", "salas", "salce", "salda", "saldo", "salga", "salgo",
    "salio", "salir", "salmón", "salon", "salpa", "salpe", "salsa", "salso", "salta",
    "salte", "salto", "salud", "salve", "salvo", "samba", "samby", "samel", "samio",
    "sampa", "sanco", "sandro", "sango", "sanar", "sanco", "sándvi", "saned", "sango",
    "sanja", "sanje", "sansa", "santa", "santo", "sapes", "saque", "sarar", "sardas",
    "sarde", "sarlo", "sarmo", "sarza", "sátir", "sauce", "sauco", "sauen", "sauna",
    "savia", "sayal", "sayas", "sayo", "sazón", "sebes", "sebor", "secar", "seco",
    "secta", "secua", "sedal", "sedar", "sedea", "sedeo", "sedes", "sedoso", "seduz",
    "segar", "segur", "seico", "seise", "seiso", "sejal", "sele", "sello", "semej",
    "senal", "senar", "senda", "sendo", "senil", "senos", "sentí", "seora", "sepan",
    "sepo", "sepos", "septi", "serbi", "seria", "serio", "serna", "serón", "seros",
    "serpo", "serra", "serre", "serru", "servo", "sesga", "sesgo", "sesio", "sesteo",
    "sesto", "sestro", "setal", "seto", "sevill", "sexmo", "sexta", "sexto", "sexuad",
    "sfera", "sforz", "siare", "sibar", "sibil", "sibila", "siclo", "sicof", "sicom",
    "sidón", "siena", "sieno", "sifon", "sigla", "siglo", "sigma", "signo", "siles",
    "silfo", "silga", "silice", "silic", "silio", "silla", "silo", "silva", "simal",
    "simar", "simio", "simón", "simple", "simpli", "sinay", "sincro", "sinfó", "singa",
    "sinod", "sinos", "sinparable", "sinsu", "sinte", "sinto", "sírio", "sirio",
    "sirle", "sirva", "sirvo", "sisar", "sisas", "sisear", "siseo", "sises", "sisla",
    "sismo", "sista", "siste", "sisto", "sitio", "sitzia", "sivel", "sobaj", "sobar",
    "sobeo", "sobil", "sobio", "sobor", "sobrinos", "soceo", "socia", "socio",
    "sodio", "soez", "soga", "sogno", "soibi", "solaz", "soler", "soles", "solev",
    "soleá", "solfa", "solic", "solio", "sollé", "sollo", "somal", "somas", "sombr",
    "somos", "sonaj", "sonar", "sonda", "sonde", "sones", "sonet", "sonio", "sonos",
    "sonro", "sonso", "sopar", "sopeo", "sopla", "sople", "soplo", "sopon", "soportal",
    "soral", "sorba", "sorbo", "sorda", "sordo", "sorgo", "sorna", "sorno", "soro",
    "soror", "sorpre", "sosia", "soso", "sotal", "sotil", "soto", "soval", "sovié",
    "soyez", "oye", "oyos", "ozono", "pabla", "pablo", "pacay", "pace", "pacer", "pacha",
    "pacte", "padre", "pafia", "pagar", "pagel", "pagro", "pague", "paico", "paila",
    "paine", "paira", "paise", "pajes", "palas", "palau", "palco", "palea", "paleo",
    "palio", "palma", "palme", "palmo", "palo", "palos", "palpa", "palpe", "palpo",
    "pampa", "panal", "panca", "pancho", "pandi", "panel", "panes", "pansa", "pantar",
    "pante", "papar", "papel", "papión", "papir", "papos", "paque", "parar", "parca",
    "parce", "parco", "parda", "pardo", "pared", "paria", "parir", "paros", "parpa",
    "parpe", "parra", "parré", "parte", "parto", "pasar", "pasas", "paseo", "pasmo",
    "paso", "paspas", "pasta", "pasto", "pata", "patán", "pateo", "patia", "pato",
    "patos", "patra", "paule", "pauli", "pausa", "pauta", "paved", "pavón", "pavos",
    "payar", "payas", "payo", "peaje", "peal", "pecar", "pecez", "pecha", "peche",
    "pedal", "pedir", "pedre", "pegar", "pegas", "pegma", "pegos", "peine", "peje",
    "pella", "pello", "pelmazo", "penal", "pene", "penes", "pengo", "penol", "pensa",
    "pense", "penso", "peón", "peor", "peper", "pepin", "peque", "peral", "perca",
    "percha", "perde", "perdi", "perej", "peret", "perla", "perno", "peros", "perse",
    "persa", "perse", "perú", "pesad", "pesar", "pesce", "pesco", "peses", "pesga",
    "pesgo", "pesos", "peste", "petar", "petas", "petau", "peteo", "petit", "petra",
    "petro", "petul", "peuco", "peumo", "peusa", "peven", "pezón", "piantar", "piar",
    "pica", "picar", "picaso", "picha", "piche", "pichel", "picho", "pico", "picor",
    "picos", "picra", "picul", "piden", "pieza", "pifano", "pifia", "pigre", "pigro",
    "pilar", "pilas", "pileo", "pilla", "pillo", "piloto", "piltra", "pimba", "pimia",
    "pimio", "pínam", "pinar", "pinco", "pindo", "pineo", "pinos", "pinta", "pinte",
    "pinto", "pinza", "piña", "piñar", "piñon", "piola", "pion", "pior", "pipa",
    "piper", "pipio", "pipir", "pipis", "pipón", "pique", "pira", "pirar", "piren",
    "pirlé", "piro", "pirol", "pirot", "pirri", "pirul", "pisar", "piscis", "pisco",
    "piseo", "pisos", "pista", "pisto", "pitón", "pitio", "pitir", "pito", "piton",
    "pitos", "pivote", "place", "placi", "plaga", "plago", "plana", "plano", "planta",
    "plata", "plato", "playa", "plazo", "plebe", "pleca", "plego", "plena", "pleno",
    "plica", "plomo", "pluma", "plume", "plumo", "plunk", "plusu", "pobre", "pocas",
    "pocka", "pocos", "podar", "poder", "podio", "podón", "podre", "pogue", "poisa",
    "poise", "polar", "polea", "polen", "polio", "polir", "polis", "polka", "polo",
    "polos", "polup", "pomar", "pombo", "pompa", "ponch", "poner", "ponga", "pongo",
    "ponle", "ponzo", "pooch", "popa", "popel", "popes", "popote", "poque", "porca",
    "porco", "porel", "porfa", "porno", "porra", "porro", "porta", "porto", "posar",
    "posca", "posco", "posda", "posde", "poseer", "posho", "posit", "posos", "posta",
    "poste", "potar", "pote", "poten", "potes", "poto", "potro", "povisa", "power",
    "pozas", "prado", "praxi", "pravo", "prear", "preco", "preda", "prefe", "prego",
    "premi", "prenda", "prepo", "preso", "previ", "prima", "primo", "prín", "prion",
    "prior", "prisa", "priva", "proas", "probe", "proca", "proco", "procur", "profir",
    "prois", "prole", "promé", "prompt", "prone", "propio", "prosa", "prostr", "proto",
    "prove", "provi", "prude", "pruna", "pruno", "púa", "puar", "pubis", "puche",
    "pucia", "pudín", "pudor", "pudra", "pudre", "pufas", "pugil", "pugna", "pugnar",
    "puir", "puita", "pulir", "pulpa", "pulpo", "pulse", "pulso", "puncha", "punco",
    "pungi", "punta", "punto", "pupa", "pupar", "pupil", "pupos", "purín", "purga",
    "purgo", "puria", "puro", "puron", "puspo", "putar", "putas", "putto", "puzle",
];

/// Estado global para la seed phrase (solo durante la sesión)
static SEED_PHRASE: Mutex<Option<String>> = Mutex::new(None);

/// Número de palabras en la mnemonic
pub const SEED_WORD_COUNT: usize = 6;

/// Genera una mnemonic aleatoria de 6 palabras
pub fn generate_mnemonic() -> Result<String, String> {
    let mut rng = rand::thread_rng();
    let mut words = Vec::with_capacity(SEED_WORD_COUNT);

    for _ in 0..SEED_WORD_COUNT {
        let word = WORD_LIST
            .choose(&mut rng)
            .ok_or("Error seleccionando palabra".to_string())?;
        words.push(*word);
    }

    let phrase = words.join(" ");

    // Guardar en memoria para verificación posterior
    let mut stored = SEED_PHRASE.lock().map_err(|_| "Lock poisoned".to_string())?;
    *stored = Some(phrase.clone());

    Ok(phrase)
}

/// Deriva una clave de 32 bytes a partir de una frase semilla
pub fn derive_key_from_seed(phrase: &str) -> Result<[u8; 32], String> {
    let normalized = normalize_seed_phrase(phrase);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    Ok(key)
}

/// Verifica si una frase semilla coincide con la generada
pub fn verify_seed_phrase(phrase: &str) -> Result<bool, String> {
    let stored = SEED_PHRASE
        .lock()
        .map_err(|_| "Lock poisoned".to_string())?;
    let stored_phrase = stored
        .as_ref()
        .ok_or("No hay frase semilla en memoria".to_string())?;

    let normalized_input = normalize_seed_phrase(phrase);
    let normalized_stored = normalize_seed_phrase(stored_phrase);

    Ok(normalized_input == normalized_stored)
}

/// Normaliza una frase semilla (minúsculas, espacios múltiples → uno solo)
fn normalize_seed_phrase(phrase: &str) -> String {
    phrase
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}

/// Calcula el hash SHA-256 de la frase para persistencia
pub fn hash_seed_phrase(phrase: &str) -> String {
    let normalized = normalize_seed_phrase(phrase);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

// =======================================================================
// TESTS
// =======================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mnemonic_word_count() {
        let phrase = generate_mnemonic().unwrap();
        let words: Vec<&str> = phrase.split_whitespace().collect();
        assert_eq!(words.len(), SEED_WORD_COUNT);
    }

    #[test]
    fn test_generate_mnemonic_words_valid() {
        let phrase = generate_mnemonic().unwrap();
        for word in phrase.split_whitespace() {
            assert!(
                WORD_LIST.contains(&word),
                "Palabra '{}' no está en la word list",
                word
            );
        }
    }

    #[test]
    fn test_generate_mnemonic_uniqueness() {
        let phrase1 = generate_mnemonic().unwrap();
        let phrase2 = generate_mnemonic().unwrap();
        // Estadísticamente deberían ser diferentes
        // (probabilidad de colisión ≈ 0)
        assert_ne!(phrase1, phrase2);
    }

    #[test]
    fn test_derive_key_deterministic() {
        let phrase = "abaco abrir acero acida actas acuna";
        let key1 = derive_key_from_seed(phrase).unwrap();
        let key2 = derive_key_from_seed(phrase).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_key_different_phrases() {
        let key1 = derive_key_from_seed("abaco abrir acero acida actas acuna").unwrap();
        let key2 = derive_key_from_seed("abrir abaco acero actas acida acuna").unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_verify_seed_phrase() {
        let phrase = generate_mnemonic().unwrap();
        assert!(verify_seed_phrase(&phrase).unwrap());
    }

    #[test]
    fn test_verify_wrong_phrase() {
        let _ = generate_mnemonic().unwrap();
        assert!(!verify_seed_phrase("esta frase es incorrecta").unwrap());
    }

    #[test]
    fn test_normalize_seed_phrase() {
        let p1 = normalize_seed_phrase("Abaco Abrir ACERO");
        let p2 = normalize_seed_phrase("  abaco   abrir   acero  ");
        assert_eq!(p1, p2);
    }

    #[test]
    fn test_hash_seed_phrase_deterministic() {
        let hash1 = hash_seed_phrase("test phrase");
        let hash2 = hash_seed_phrase("test phrase");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_word_list_size() {
        assert!(WORD_LIST.len() >= 256, "Word list debe tener al menos 256 palabras");
    }
}
