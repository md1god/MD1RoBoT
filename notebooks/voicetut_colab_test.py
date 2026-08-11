# ============================================
# VoiceTut-TTS — تحسين النطق بالأدوات الرسمية (بدون تدريب)
# ============================================
# الخطوات:
# 1. افتح https://colab.research.google.com
# 2. New Notebook
# 3. من فوق: Runtime > Change runtime type > اختار T4 GPU > Save
# 4. الصق الكود ده في خلية واحدة أو أكتر وشغله بالترتيب
#
# ملحوظة: ده بيستخدم ArabicNormalizer + add_lexicon الرسميين من نفس
# مكتبة voicetut-tts، بدل أي تعديل يدوي على temperature/top_p.

# ---------- الخلية 1: تثبيت المكتبات ----------
!pip install torch --index-url https://download.pytorch.org/whl/cu121
!pip install git+https://github.com/k2-fsa/OmniVoice.git
!pip install voicetut-tts

# ---------- الخلية 2: تحميل الموديل ----------
from voicetut_tts import VoiceTutTTS

tts = VoiceTutTTS.from_pretrained("mohammedaly22/VoiceTut-TTS")
print("الموديل اتحمل بنجاح")

# ---------- الخلية 3: توليد صوت بصوت جاهز ----------
tts.synthesize(
    "ازيك عامل ايه النهاردة؟ ده اختبار للصوت المصري",
    speaker="Mohamed",   # ممكن تجرب: Asmaa, Sayed, وغيرهم من الـ 15 صوت
    output="test1.wav"
)

# تشغيل الصوت مباشرة جوه Colab
from IPython.display import Audio
Audio("test1.wav")

# ---------- الخلية 4 (اختياري): جملة فيها كود سويتشينج عربي/إنجليزي ----------
tts.synthesize(
    "عندي meeting بكرة الساعة 3:30 الظهر",
    speaker="Asmaa",
    output="test2.wav"
)
Audio("test2.wav")

# ---------- الخلية 5 (اختياري): قياس زمن التوليد الفعلي ----------
import time
start = time.time()
tts.synthesize("النهارده الجو حلو اوي وعايز اطلع اتمشى", speaker="Sayed", output="test3.wav")
elapsed = time.time() - start
print(f"استغرق التوليد: {elapsed:.2f} ثانية")
Audio("test3.wav")

# ---------- الخلية 6: Zero-shot voice cloning من ملف مرجعي (ميمي) ----------
# ارفع ملف "ميمي" لجلسة Colab الأول:
# من القائمة الجانبية اليسرى -> أيقونة الملفات -> Upload -> اختار الملف
# لازم يكون الملف .wav أو .mp3، وطوله من 5 لـ 15 ثانية كفاية
#
# ملحوظة تقنية: الـ API الرسمي محتاج ref_text (نص مطابق للي قايل في الملف
# المرجعي بالظبط) مش بس ref_audio. من غيره الموديل مش هيعرف يوازن الصوت صح.

ref_audio_mimi = "/content/mimi.wav"          # عدّل المسار حسب اسم اللي رفعته
ref_text_mimi  = "اكتب هنا بالظبط اللي ميمي بتقوله في المقطع"  # النص المطابق للملف

tts.synthesize(
    "ازيك يا حبيبي، عامل ايه النهاردة؟ عايز نلعب مع بعض",
    ref_audio=ref_audio_mimi,
    ref_text=ref_text_mimi,
    output="test_mimi.wav"
)
Audio("test_mimi.wav")

# ---------- الخلية 7: نفس الفكرة بصوت سوسو ----------
ref_audio_sosO = "/content/sosO.wav"          # عدّل المسار حسب اسم اللي رفعته
ref_text_sosO  = "اكتب هنا بالظبط اللي سوسو بتقوله في المقطع"

start = time.time()
tts.synthesize(
    "الشمس طلعت وحصانك جاهز يا بطل",
    ref_audio=ref_audio_sosO,
    ref_text=ref_text_sosO,
    output="test_sosO.wav"
)
elapsed = time.time() - start
print(f"استغرق التوليد بالاستنساخ: {elapsed:.2f} ثانية")
Audio("test_sosO.wav")

# ---------- الخلية 8: تحسين النطق بالأداة الرسمية (ArabicNormalizer) ----------
# ده الحل الرسمي لمشكلة "كلمة واحدة نطقها غلط" -- بدل ما نلعب في
# temperature/top_p، بنصلح النطق مباشرة بالتشكيل.

from voicetut_tts import ArabicNormalizer

norm = ArabicNormalizer()

# مثال: لو "توت" كانت بتتنطق غلط، نضيفها بالتشكيل الصحيح
norm.add_lexicon({"توت": "تُوت"})   # عدّل الكلمة والتشكيل حسب اللي غلط عندك

# جرب الجملة اللي فيها الكلمة اللي كانت مشكلة
fixed_text = norm("النص اللي فيه الكلمة اللي كانت غلط")
print("النص بعد التصحيح:", fixed_text)

tts.synthesize(
    fixed_text,
    ref_audio=ref_audio_mimi,
    ref_text=ref_text_mimi,
    output="test_fixed_pronunciation.wav"
)
Audio("test_fixed_pronunciation.wav")

# ---------- الخلية 9 (اختياري): إضافة كذا كلمة دفعة واحدة ----------
# لو فيه أكتر من كلمة بتتنطق غلط، ضيفهم كلهم هنا مرة واحدة
norm.add_lexicon({
    "توت": "تُوت",
    # "كلمة تانية": "تشكيلها الصح",
})
