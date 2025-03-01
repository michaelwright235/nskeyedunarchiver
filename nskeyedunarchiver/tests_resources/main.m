#import <Foundation/Foundation.h>
#import <AppKit/AppKit.h>

@interface Note : NSObject <NSCoding> {
  NSString *title;
  NSString *author;
  BOOL published;
  NSArray *array;
}

@property (nonatomic, copy) NSString *title;
@property (nonatomic, copy) NSString *author;
@property (nonatomic) BOOL published;
@property (nonatomic, copy) NSArray *array;

@end

@implementation Note

@synthesize title;
@synthesize author;
@synthesize published;
@synthesize array;

- (void)dealloc {
  [title release];
  [author release];
  [super dealloc];
}

- (id)initWithCoder:(NSCoder *)decoder {
  if (self = [super init]) {
    self.title = [decoder decodeObjectForKey:@"title"];
    self.author = [decoder decodeObjectForKey:@"author"];
    self.published = [decoder decodeBoolForKey:@"published"];
    self.array = [decoder decodeObjectForKey:@"array"];
  }
  return self;
}

- (void)encodeWithCoder:(NSCoder *)encoder {
  [encoder encodeObject:title forKey:@"title"];
  [encoder encodeObject:author forKey:@"author"];
  [encoder encodeBool:published forKey:@"published"];
  [encoder encodeObject:array forKey:@"array"];
}

@end


void archiveData(id dict, NSString* name) {
    NSError* arcError = nil;
    NSData* data = [NSKeyedArchiver
     archivedDataWithRootObject: dict
     requiringSecureCoding: NO
     error: &arcError
    ];
    if(arcError != nil) {
        NSLog(@"Archiver error: %@", [arcError localizedDescription]);
    }

    NSError *writingError = nil;
    NSString* path= [NSString stringWithFormat: @"./plists/%@.plist", name];
    [data writeToFile:path options:NSDataWritingAtomic error:&writingError];
    if(writingError != nil) {
        NSLog(@"Writing error: %@", [writingError localizedDescription]);
    }
}

void bundle(void) {
    NSMutableAttributedString *string = [[NSMutableAttributedString alloc] initWithString:@"firstsecondthird"];
    [string addAttribute:NSForegroundColorAttributeName value:[NSColor redColor] range:NSMakeRange(0,5)];
    archiveData(string, @"NSMutableAttributedString");
}

void archiveNSAffineTransform(void) {
    NSAffineTransform* a = [[NSAffineTransform alloc] init];
    [a rotateByDegrees: 15.5];
    archiveData(a, @"NSAffineTransform");
}

void plainString(void) {
    NSString* str = @"Some string!";
    archiveData(str,  @"plainString");
}

void nsData(void) {
    NSString* str = @"Some data!";
    NSData* data = [str dataUsingEncoding:NSUTF8StringEncoding];
    archiveData(data,  @"nsData");
}

void simpleArray(void) {
    NSArray *innerArray = [NSArray arrayWithObjects:
        @"innerValue3",
        @"innerValue4",
        nil];
    NSArray *myArray = [NSArray arrayWithObjects:
        @"value1",
        @"value2",
        innerArray, nil];
    archiveData(myArray, @"simpleArray");
}

void simpleDict(void) {
    NSDictionary *dict = @{
        @"First key":@"First value",
        @"Second key":@"Second value",
        @"Array key":@[@1,@2,@3]
    };
    archiveData(dict, @"simpleDict");
}

void circularReference(void) {
    NSMutableArray *array = [NSMutableArray arrayWithObjects:
    @"self reference here",
    nil];
    array[0] = array;
    archiveData(array, @"circularReference");
}

void note(void) {
    Note *object = [[Note alloc] init];
    object.title = @"Some cool title";
    object.author = @"Michael Wright";
    object.published = TRUE;
    object.array = @[@"Hello, World!", @42, @YES];
    archiveData(object, @"note");
}

int main(int argc, const char * argv[]) {
    @autoreleasepool {
        plainString();
        nsData();
        simpleArray();
        simpleDict();
        circularReference();
        archiveNSAffineTransform();
        bundle();
        note();
    }
    return 0;
}
