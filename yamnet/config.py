# Training params
EPOCHS = 20
BATCH_SIZE = 64
VALIDATION_RATIO = 0.1

# How many files to load per dataset type
DATASET_LIMIT = 1000000

# Random seed for all operations
SEED = 1234568

# Location where the dataset will be downloaded.
# By default (None), keras.utils.get_file will use ~/.keras/ as the CACHE_DIR
CACHE_DIR = None

# Model definitions
# CLASS_ID maps dataset type to label

# Adverts VS music+talk, too BAD
MODEL_NAME = "adbanda_a_mt"
CLASS_NAMES = ["advert", "music_talk"]
CLASS_ID = [0, 1, 1]

# Adverts + Talks VS music ??
# MODEL_NAME = "adbanda_at_m"
# CLASS_NAMES = ["advert_talk", "music"]
# CLASS_ID = [0, 1, 0]

# Adverts VS music VS talk, so far VERY BAD
# MODEL_NAME = 'adbanda_a_m_t'
# CLASS_NAMES = ['advert', 'music', 'talk']
# CLASS_ID = [0, 1, 2]
