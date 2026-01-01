export interface RandomThemeConfig {
  theme: string;
  genres?: string[];
  synopsis?: string;
  characters?: Array<{
    name: string;
    gender: string;
    description: string;
    isMain: boolean;
  }>;
}

export const randomThemes: RandomThemeConfig[] = [
  {
    theme: '升职前一晚',
    genres: ['职场', '剧情', '悬疑'],
    synopsis:
      '深夜十一点，办公室只剩下我还在加班。明天就是期待已久的部门经理面试。整理文件时，一份标注"机密"的文件夹掉了出来——里面竟然有王总挪用公款的证据。这时，走廊里传来了脚步声，我必须立刻做出选择：举报还是沉默？这份证据或许是我升职的筹码，但也可能让我万劫不复。',
    characters: [
      {
        name: '陈默',
        gender: '男',
        description:
          '28岁，普通职员。渴望改变现状，正直但面临诱惑，手握王总挪用公款的证据。',
        isMain: true,
      },
      {
        name: '王总',
        gender: '男',
        description:
          '45岁，公司总经理。表面和蔼，实则手段狠辣，挪用公款的秘密一旦暴露将身败名裂。',
        isMain: false,
      },
      {
        name: '李雅琪',
        gender: '女',
        description:
          '27岁，竞争对手。聪明善妒，为达目的不择手段，似乎也察觉到了什么。',
        isMain: false,
      },
      {
        name: '老张',
        gender: '男',
        description:
          '50岁，老员工。知道公司内幕但明哲保身，关键时刻可能成为盟友或障碍。',
        isMain: false,
      },
    ],
  },
  {
    theme: '重生之被班主任拍醒',
    genres: ['青春', '喜剧', '剧情'],
    synopsis:
      '数学课，我趴在课桌上流口水。突然，后脑勺挨了一巴掌："又睡觉！"猛一抬头，我惊恐地发现——黑板上写着"2014年"，眼前是年轻十岁的班主任老张。我竟然重生了！带着30岁社畜的记忆回到高三，我发誓要改变高考失利的命运，重新追回错过的暗恋，但事情真的会如我所愿吗？',
    characters: [
      {
        name: '林小北',
        gender: '男',
        description:
          '18岁，高三学生（内心30岁）。带着未来的记忆重生，决心改变命运，但成人思维与少年身份格格不入。',
        isMain: true,
      },
      {
        name: '张老师',
        gender: '男',
        description:
          '40岁，数学老师兼班主任。严厉古板，口头禅"你们是我带过最差的一届"，其实关心学生。',
        isMain: false,
      },
      {
        name: '苏小美',
        gender: '女',
        description:
          '18岁，同桌。当年的暗恋对象，活泼可爱，数学很差但很努力，不知道自己已被"预支"了未来。',
        isMain: false,
      },
      {
        name: '胖子大伟',
        gender: '男',
        description:
          '18岁，死党。讲义气但成绩烂，未来会出国发展，现在是需要帮助的好兄弟。',
        isMain: false,
      },
    ],
  },
  {
    theme: '电梯停在中间',
    genres: ['悬疑', '惊悚', '剧情'],
    synopsis:
      '加班到凌晨两点，我独自走进公司电梯。按下一楼的按钮，电梯却缓缓上行...停在了一个不存在的楼层。电梯门缓缓打开，外面是一片漆黑，只有远处传来诡异的哭声。手机没有信号，电梯门关不上，我被迫走出电梯，探索这个隐藏在写字楼中的诡异空间。这里到底隐藏着什么秘密？',
    characters: [
      {
        name: '周远',
        gender: '男',
        description:
          '26岁，程序员。深夜加班被困诡异楼层，手机无信号，必须找到出路逃离。',
        isMain: true,
      },
      {
        name: '保安老王',
        gender: '男',
        description:
          '60岁，夜班保安。神神叨叨，似乎知道这栋楼的秘密，但说话总是语焉不详。',
        isMain: false,
      },
      {
        name: '神秘女子',
        gender: '女',
        description:
          '身份不明的长发女子，在黑暗中若隐若现。似乎在求助，又可能在引诱人陷入陷阱。',
        isMain: false,
      },
      {
        name: '陈经理',
        gender: '男',
        description:
          '35岁，三年前在同一楼层失踪的前经理。官方说法是离职跳槽，真相成谜。',
        isMain: false,
      },
    ],
  },
  {
    theme: '让键盘飞',
    genres: ['喜剧', '职场', '奇幻'],
    synopsis:
      '作为一名程序员，我发现我的键盘突然有了超能力——敲下的代码能控制现实世界！输入"删除"，项目经理的头发真的消失了；输入"bug"，老板的腿突然一瘸。我成了公司里的"神"，但很快发现，每次使用能力，我的寿命就会减少一天。当能力与生命画上等号，我该如何抉择？',
    characters: [
      {
        name: '码农阿强',
        gender: '男',
        description:
          '25岁，程序员。意外获得能控制现实的键盘，从受气包变成了公司里的"神"，但代价是寿命。',
        isMain: true,
      },
      {
        name: 'Jerry',
        gender: '男',
        description:
          '30岁，产品经理。总是提出"五彩斑斓的黑"这种离谱需求，是阿强最先"报复"的对象。',
        isMain: false,
      },
      {
        name: '老刘',
        gender: '男',
        description:
          '40岁，技术总监。技术过硬但耳根子软，可能会发现阿强的秘密，成为关键转折点。',
        isMain: false,
      },
      {
        name: '小萌',
        gender: '女',
        description:
          '26岁，HR。温柔善良，暗恋阿强，可能会成为阿强的软肋或救赎。',
        isMain: false,
      },
    ],
  },
  {
    theme: '赛博程序员 2077',
    genres: ['科幻', '动作', '赛博朋克'],
    synopsis:
      '2077年，夜之城。作为一名地下黑客程序员，我接到了一个看似普通的任务：潜入荒坂塔，取回一份被盗窃的代码。但很快发现，这份代码里隐藏着能让整个城市瘫痪的AI病毒。荒坂公司的杀手已经开始追踪我，而联络人艾丽丝似乎也有自己的目的。在这座霓虹闪烁的罪恶之城中，我该如何生存？',
    characters: [
      {
        name: '零号',
        gender: '男',
        description:
          '24岁，地下黑客。技艺高超但身无分文，为了妹妹的医药费接下荒坂任务，却意外卷入巨大阴谋。',
        isMain: true,
      },
      {
        name: '艾丽丝',
        gender: '女',
        description:
          '28岁，神秘联络人。自称反抗组织成员，美丽而危险，真实身份成谜，可能与荒坂有关。',
        isMain: false,
      },
      {
        name: '荒坂健',
        gender: '男',
        description:
          '32岁，荒坂公司杀手。半机械化改造，冷酷无情，一直追踪主角，不死不休。',
        isMain: false,
      },
      {
        name: '小零',
        gender: '女',
        description:
          '16岁，零号的妹妹。患有罕见基因病，需要昂贵的纳米药物治疗，是零号的一切动力来源。',
        isMain: false,
      },
    ],
  },
  {
    theme: '前任5:初为人父',
    genres: ['剧情', '家庭', '都市'],
    synopsis:
      '分手五年后，她突然抱着一个四岁的孩子出现在我家门口："这是你的孩子。"DNA检测证实了她的说法。我不得不开始学习如何做一个父亲——换尿布、冲奶粉、讲故事。而她的重新出现也让我和现任女友赵婷的关系陷入危机。在责任、爱情和亲情之间，我该如何抉择？',
    characters: [
      {
        name: '林浩',
        gender: '男',
        description:
          '30岁，广告公司总监。事业有成但感情空虚，突然被告知有个孩子，手忙脚乱地学习当父亲。',
        isMain: true,
      },
      {
        name: '苏晴',
        gender: '女',
        description:
          '28岁，前女友。五年前分手后独自抚养孩子，如今走投无路才找上门，内心仍有未了的情感。',
        isMain: false,
      },
      {
        name: '小宝',
        gender: '男',
        description:
          '4岁，林浩和苏晴的儿子。天真可爱，不知道父母之间的复杂关系，是无辜的受害者。',
        isMain: false,
      },
      {
        name: '赵婷',
        gender: '女',
        description:
          '29岁，林浩现女友。温柔体贴，原本计划结婚，突然出现的"前女友和孩子"让她措手不及。',
        isMain: false,
      },
    ],
  },
  {
    theme: '千杯不醉',
    genres: ['喜剧', '悬疑', '都市'],
    synopsis:
      '今晚是高中同学聚会，我发誓一定要清醒回家。但一杯接一杯后，世界开始旋转...醒来时，我发现自己竟然在陌生的酒店房间，床边放着一叠现金，而我完全不记得昨晚发生了什么。手机里有一条未读短信："谢了，钱收到了。"我到底做了什么？这叠钱从哪来？我必须找回失落的记忆。',
    characters: [
      {
        name: '阿明',
        gender: '男',
        description:
          '28岁，普通上班族。酒量很差但爱面子，同学聚会喝断片后陷入谜团，必须找回记忆。',
        isMain: true,
      },
      {
        name: '老周',
        gender: '男',
        description:
          '29岁，聚会组织者。当年暗恋班花，如今事业有成，但似乎有什么秘密在隐瞒。',
        isMain: false,
      },
      {
        name: '小雪',
        gender: '女',
        description:
          '28岁，当年班花。婚姻不幸，聚会当晚喝醉后与主角有某些互动，可能是关键线索。',
        isMain: false,
      },
      {
        name: '大伟',
        gender: '男',
        description:
          '28岁，主角死党。声称当晚送主角回家，但说法前后矛盾，似乎在隐瞒什么。',
        isMain: false,
      },
    ],
  },
];

export function getThemeConfig(
  themeName: string,
): RandomThemeConfig | undefined {
  return randomThemes.find((t) => t.theme === themeName);
}
